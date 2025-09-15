// Message Passing System
//
// Provides inter-module communication through message passing

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc, broadcast};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// Message that can be sent between modules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub source: String,
    pub target: String,
    pub message_type: MessageType,
    pub payload: Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub correlation_id: Option<String>,
    pub reply_to: Option<String>,
}

impl Message {
    /// Create a new message
    pub fn new(source: String, target: String, message_type: MessageType, payload: Value) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            source,
            target,
            message_type,
            payload,
            timestamp: chrono::Utc::now(),
            correlation_id: None,
            reply_to: None,
        }
    }

    /// Create a reply to another message
    pub fn reply_to(original: &Message, payload: Value) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            source: original.target.clone(),
            target: original.source.clone(),
            message_type: MessageType::Response,
            payload,
            timestamp: chrono::Utc::now(),
            correlation_id: original.correlation_id.clone(),
            reply_to: Some(original.id.clone()),
        }
    }

    /// Set correlation ID for tracking related messages
    pub fn with_correlation_id(mut self, id: String) -> Self {
        self.correlation_id = Some(id);
        self
    }
}

/// Types of messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    Request,
    Response,
    Event,
    Command,
    Query,
    Notification,
    Error,
    Broadcast,
}

/// Handler for processing messages
#[async_trait::async_trait]
pub trait MessageHandler: Send + Sync {
    /// Handle an incoming message
    async fn handle_message(&mut self, message: Message) -> Result<Option<Message>>;

    /// Get supported message types
    fn supported_types(&self) -> Vec<MessageType>;

    /// Check if a message type is supported
    fn supports(&self, message_type: &MessageType) -> bool {
        self.supported_types().iter().any(|t| std::mem::discriminant(t) == std::mem::discriminant(message_type))
    }
}

/// Message subscription
struct Subscription {
    id: String,
    filter: Box<dyn Fn(&Message) -> bool + Send + Sync>,
    sender: mpsc::UnboundedSender<Message>,
}

/// Message bus for routing messages between modules
pub struct MessageBus {
    subscriptions: Arc<RwLock<HashMap<String, Vec<Subscription>>>>,
    broadcast_channel: broadcast::Sender<Message>,
    message_history: Arc<RwLock<Vec<Message>>>,
    max_history_size: usize,
}

impl MessageBus {
    /// Create a new message bus
    pub fn new() -> Self {
        let (broadcast_tx, _) = broadcast::channel(1000);
        
        Self {
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            broadcast_channel: broadcast_tx,
            message_history: Arc::new(RwLock::new(Vec::new())),
            max_history_size: 1000,
        }
    }

    /// Send a message to a specific target
    pub async fn send(&self, target: &str, message: Message) -> Result<()> {
        // Store in history
        self.add_to_history(message.clone()).await;

        // Send to specific target subscriptions
        let subs = self.subscriptions.read().await;
        if let Some(target_subs) = subs.get(target) {
            for sub in target_subs {
                if (sub.filter)(&message) {
                    let _ = sub.sender.send(message.clone());
                }
            }
        }

        // Send to wildcard subscriptions
        if let Some(wildcard_subs) = subs.get("*") {
            for sub in wildcard_subs {
                if (sub.filter)(&message) {
                    let _ = sub.sender.send(message.clone());
                }
            }
        }

        Ok(())
    }

    /// Broadcast a message to all subscribers
    pub async fn broadcast(&self, mut message: Message) -> Result<()> {
        message.message_type = MessageType::Broadcast;
        
        // Store in history
        self.add_to_history(message.clone()).await;

        // Send through broadcast channel
        let _ = self.broadcast_channel.send(message.clone());

        // Also send to all direct subscriptions
        let subs = self.subscriptions.read().await;
        for (_, target_subs) in subs.iter() {
            for sub in target_subs {
                if (sub.filter)(&message) {
                    let _ = sub.sender.send(message.clone());
                }
            }
        }

        Ok(())
    }

    /// Subscribe to messages for a specific target
    pub async fn subscribe(
        &self,
        target: String,
        filter: Box<dyn Fn(&Message) -> bool + Send + Sync>,
    ) -> mpsc::UnboundedReceiver<Message> {
        let (tx, rx) = mpsc::unbounded_channel();
        
        let subscription = Subscription {
            id: Uuid::new_v4().to_string(),
            filter,
            sender: tx,
        };

        let mut subs = self.subscriptions.write().await;
        subs.entry(target).or_insert_with(Vec::new).push(subscription);

        rx
    }

    /// Subscribe to broadcast messages
    pub fn subscribe_broadcast(&self) -> broadcast::Receiver<Message> {
        self.broadcast_channel.subscribe()
    }

    /// Request-response pattern
    pub async fn request(&self, target: &str, message: Message) -> Result<Message> {
        let reply_channel = self.subscribe(
            message.source.clone(),
            Box::new({
                let msg_id = message.id.clone();
                move |m| m.reply_to.as_ref() == Some(&msg_id)
            }),
        ).await;

        // Send the request
        self.send(target, message).await?;

        // Wait for response with timeout
        tokio::time::timeout(
            std::time::Duration::from_secs(5),
            async {
                let mut receiver = reply_channel;
                receiver.recv().await
                    .ok_or_else(|| anyhow::anyhow!("No response received"))
            },
        ).await
            .map_err(|_| anyhow::anyhow!("Request timed out"))?
    }

    /// Get message history
    pub async fn get_history(&self, limit: Option<usize>) -> Vec<Message> {
        let history = self.message_history.read().await;
        let limit = limit.unwrap_or(history.len());
        
        history.iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    /// Get messages by correlation ID
    pub async fn get_by_correlation_id(&self, correlation_id: &str) -> Vec<Message> {
        self.message_history.read().await
            .iter()
            .filter(|m| m.correlation_id.as_ref() == Some(&correlation_id.to_string()))
            .cloned()
            .collect()
    }

    /// Add message to history
    async fn add_to_history(&self, message: Message) {
        let mut history = self.message_history.write().await;
        history.push(message);
        
        // Trim history if it exceeds max size
        if history.len() > self.max_history_size {
            let excess = history.len() - self.max_history_size;
            history.drain(0..excess);
        }
    }

    /// Clear message history
    pub async fn clear_history(&self) {
        self.message_history.write().await.clear();
    }

    /// Get statistics about message bus usage
    pub async fn get_stats(&self) -> MessageBusStats {
        let history = self.message_history.read().await;
        let subscriptions = self.subscriptions.read().await;

        let mut type_counts = HashMap::new();
        for msg in history.iter() {
            *type_counts.entry(format!("{:?}", msg.message_type)).or_insert(0) += 1;
        }

        MessageBusStats {
            total_messages: history.len(),
            subscription_count: subscriptions.values().map(|v| v.len()).sum(),
            message_type_counts: type_counts,
        }
    }
}

/// Statistics about message bus usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageBusStats {
    pub total_messages: usize,
    pub subscription_count: usize,
    pub message_type_counts: HashMap<String, usize>,
}

/// Simple message router for direct module-to-module communication
pub struct MessageRouter {
    handlers: Arc<RwLock<HashMap<String, Box<dyn MessageHandler>>>>,
}

impl MessageRouter {
    /// Create a new message router
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a message handler for a module
    pub async fn register_handler(&self, module_id: String, handler: Box<dyn MessageHandler>) {
        self.handlers.write().await.insert(module_id, handler);
    }

    /// Unregister a message handler
    pub async fn unregister_handler(&self, module_id: &str) {
        self.handlers.write().await.remove(module_id);
    }

    /// Route a message to the appropriate handler
    pub async fn route(&self, message: Message) -> Result<Option<Message>> {
        let mut handlers = self.handlers.write().await;
        
        if let Some(handler) = handlers.get_mut(&message.target) {
            if handler.supports(&message.message_type) {
                handler.handle_message(message).await
            } else {
                anyhow::bail!("Handler does not support message type: {:?}", message.message_type)
            }
        } else {
            anyhow::bail!("No handler registered for target: {}", message.target)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = Message::new(
            "source".to_string(),
            "target".to_string(),
            MessageType::Request,
            Value::String("test".to_string()),
        );

        assert_eq!(msg.source, "source");
        assert_eq!(msg.target, "target");
        assert!(matches!(msg.message_type, MessageType::Request));
    }

    #[test]
    fn test_message_reply() {
        let original = Message::new(
            "sender".to_string(),
            "receiver".to_string(),
            MessageType::Request,
            Value::Null,
        );

        let reply = Message::reply_to(&original, Value::String("response".to_string()));

        assert_eq!(reply.source, "receiver");
        assert_eq!(reply.target, "sender");
        assert_eq!(reply.reply_to, Some(original.id));
    }

    #[tokio::test]
    async fn test_message_bus() {
        let bus = MessageBus::new();
        
        let mut receiver = bus.subscribe(
            "test_module".to_string(),
            Box::new(|_| true),
        ).await;

        let msg = Message::new(
            "sender".to_string(),
            "test_module".to_string(),
            MessageType::Notification,
            Value::String("hello".to_string()),
        );

        bus.send("test_module", msg.clone()).await.unwrap();

        let received = receiver.recv().await.unwrap();
        assert_eq!(received.id, msg.id);
    }
}