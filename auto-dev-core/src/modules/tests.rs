// Module System Tests
//
// Comprehensive tests for the dynamic module system

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::modules::interface::{
        ModuleInterface, ModuleMetadata, ModuleCapability, ModuleState, ModuleVersion
    };
    use crate::modules::loader::{ModuleLoader, ModuleFormat};
    use crate::modules::registry::{ModuleRegistry, ModuleStatus};
    use crate::modules::runtime::{ModuleRuntime, ExecutionContext};
    use crate::modules::messages::{Message, MessageBus, MessageType};
    use async_trait::async_trait;
    use serde_json::Value;
    use anyhow::Result;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    /// Mock module for testing
    struct MockModule {
        metadata: ModuleMetadata,
        state: ModuleState,
        execution_count: Arc<RwLock<u64>>,
        initialized: bool,
    }

    impl MockModule {
        fn new(name: &str) -> Self {
            let metadata = ModuleMetadata {
                name: name.to_string(),
                version: ModuleVersion::new(1, 0, 0),
                author: "test".to_string(),
                description: "Mock module for testing".to_string(),
                capabilities: vec![
                    ModuleCapability::Custom {
                        name: "test".to_string(),
                        description: "Test capability".to_string(),
                    },
                ],
                dependencies: vec![],
            };

            Self {
                metadata: metadata.clone(),
                state: ModuleState::new(metadata.version),
                execution_count: Arc::new(RwLock::new(0)),
                initialized: false,
            }
        }
    }

    #[async_trait]
    impl ModuleInterface for MockModule {
        fn metadata(&self) -> ModuleMetadata {
            self.metadata.clone()
        }

        async fn initialize(&mut self) -> Result<()> {
            self.initialized = true;
            Ok(())
        }

        async fn execute(&self, input: Value) -> Result<Value> {
            let mut count = self.execution_count.write().await;
            *count += 1;
            
            Ok(serde_json::json!({
                "input": input,
                "execution_count": *count,
                "module": self.metadata.name,
            }))
        }

        fn get_capabilities(&self) -> Vec<ModuleCapability> {
            self.metadata.capabilities.clone()
        }

        async fn handle_message(&mut self, message: Value) -> Result<Option<Value>> {
            Ok(Some(serde_json::json!({
                "received": message,
                "module": self.metadata.name,
            })))
        }

        async fn shutdown(&mut self) -> Result<()> {
            self.initialized = false;
            Ok(())
        }

        fn get_state(&self) -> Result<ModuleState> {
            Ok(self.state.clone())
        }

        fn restore_state(&mut self, state: ModuleState) -> Result<()> {
            self.state = state;
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_module_system_lifecycle() {
        let system = ModuleSystem::new().unwrap();
        
        // Initially no modules
        let modules = system.list_modules().await.unwrap();
        assert_eq!(modules.len(), 0);
    }

    #[tokio::test]
    async fn test_module_registry() {
        let _registry = ModuleRegistry::new();
        
        // Create mock module
        let module = MockModule::new("test_module");
        let _metadata = module.metadata();
        
        // In real tests, we would use actual module files
        // This test demonstrates the registry API structure
    }

    #[tokio::test]
    async fn test_module_versioning() {
        let v1 = ModuleVersion::new(1, 0, 0);
        let v2 = ModuleVersion::new(1, 1, 0);
        let v3 = ModuleVersion::new(2, 0, 0);
        
        assert!(v1.is_compatible_with(&v2));
        assert!(!v1.is_compatible_with(&v3));
        
        let v_pre = ModuleVersion {
            major: 1,
            minor: 0,
            patch: 0,
            pre_release: Some("alpha".to_string()),
        };
        
        assert_eq!(v_pre.to_string(), "1.0.0-alpha");
    }

    #[tokio::test]
    async fn test_execution_context() {
        let context = ExecutionContext::new(Value::String("test".to_string()))
            .with_config("debug".to_string(), Value::Bool(true))
            .with_timeout(1000)
            .with_tracing(true);
        
        assert_eq!(context.timeout_ms, Some(1000));
        assert!(context.trace_enabled);
        assert_eq!(
            context.config.get("debug"),
            Some(&Value::Bool(true))
        );
    }

    #[tokio::test]
    async fn test_message_bus() {
        let bus = MessageBus::new();
        
        // Subscribe to messages
        let mut receiver = bus.subscribe(
            "test_target".to_string(),
            Box::new(|msg| matches!(msg.message_type, MessageType::Request)),
        ).await;
        
        // Send message
        let msg = Message::new(
            "sender".to_string(),
            "test_target".to_string(),
            MessageType::Request,
            serde_json::json!({"data": "test"}),
        );
        
        bus.send("test_target", msg.clone()).await.unwrap();
        
        // Receive message
        let received = receiver.recv().await.unwrap();
        assert_eq!(received.id, msg.id);
        assert_eq!(received.payload, msg.payload);
    }

    #[tokio::test]
    async fn test_message_correlation() {
        let bus = MessageBus::new();
        
        let correlation_id = "test-correlation-123";
        
        // Send correlated messages
        let msg1 = Message::new(
            "sender".to_string(),
            "target".to_string(),
            MessageType::Request,
            Value::Null,
        ).with_correlation_id(correlation_id.to_string());
        
        let msg2 = Message::reply_to(&msg1, serde_json::json!({"status": "ok"}));
        
        bus.send("target", msg1.clone()).await.unwrap();
        bus.send("sender", msg2.clone()).await.unwrap();
        
        // Get messages by correlation ID
        let correlated = bus.get_by_correlation_id(correlation_id).await;
        assert_eq!(correlated.len(), 2);
    }

    #[tokio::test]
    async fn test_message_broadcast() {
        let bus = MessageBus::new();
        
        // Subscribe to broadcasts
        let mut receiver1 = bus.subscribe_broadcast();
        let mut receiver2 = bus.subscribe_broadcast();
        
        // Broadcast message
        let msg = Message::new(
            "broadcaster".to_string(),
            "*".to_string(),
            MessageType::Broadcast,
            serde_json::json!({"announcement": "hello"}),
        );
        
        bus.broadcast(msg.clone()).await.unwrap();
        
        // Both receivers should get the message
        let received1 = receiver1.recv().await.unwrap();
        let received2 = receiver2.recv().await.unwrap();
        
        assert_eq!(received1.payload, msg.payload);
        assert_eq!(received2.payload, msg.payload);
    }

    #[tokio::test]
    async fn test_module_state_persistence() {
        let mut module = MockModule::new("stateful_module");
        
        // Set some state
        module.state.set(
            "counter".to_string(),
            Value::Number(42.into()),
        );
        
        // Get state
        let state = module.get_state().unwrap();
        assert_eq!(
            state.get("counter"),
            Some(&Value::Number(42.into()))
        );
        
        // Create new module and restore state
        let mut new_module = MockModule::new("stateful_module");
        new_module.restore_state(state).unwrap();
        
        let restored_state = new_module.get_state().unwrap();
        assert_eq!(
            restored_state.get("counter"),
            Some(&Value::Number(42.into()))
        );
    }

    #[tokio::test]
    async fn test_module_capabilities() {
        let module = MockModule::new("capability_test");
        let caps = module.get_capabilities();
        
        assert_eq!(caps.len(), 1);
        match &caps[0] {
            ModuleCapability::Custom { name, .. } => {
                assert_eq!(name, "test");
            }
            _ => panic!("Expected Custom capability"),
        }
    }

    #[tokio::test]
    async fn test_module_initialization() {
        let mut module = MockModule::new("init_test");
        
        assert!(!module.initialized);
        module.initialize().await.unwrap();
        assert!(module.initialized);
        
        module.shutdown().await.unwrap();
        assert!(!module.initialized);
    }

    #[tokio::test]
    async fn test_module_execution() {
        let module = MockModule::new("exec_test");
        
        let input = serde_json::json!({
            "command": "test",
            "params": [1, 2, 3],
        });
        
        let result = module.execute(input.clone()).await.unwrap();
        
        assert_eq!(result["input"], input);
        assert_eq!(result["module"], "exec_test");
        assert_eq!(result["execution_count"], 1);
        
        // Execute again
        let result2 = module.execute(input.clone()).await.unwrap();
        assert_eq!(result2["execution_count"], 2);
    }

    #[tokio::test]
    async fn test_message_history() {
        let bus = MessageBus::new();
        
        // Send multiple messages
        for i in 0..5 {
            let msg = Message::new(
                "sender".to_string(),
                format!("target_{}", i),
                MessageType::Notification,
                Value::Number(i.into()),
            );
            bus.send(&format!("target_{}", i), msg).await.unwrap();
        }
        
        // Check history
        let history = bus.get_history(Some(3)).await;
        assert_eq!(history.len(), 3);
        
        let all_history = bus.get_history(None).await;
        assert_eq!(all_history.len(), 5);
        
        // Clear history
        bus.clear_history().await;
        let cleared_history = bus.get_history(None).await;
        assert_eq!(cleared_history.len(), 0);
    }

    #[tokio::test]
    async fn test_message_stats() {
        let bus = MessageBus::new();
        
        // Send various message types
        bus.send("target", Message::new(
            "s".to_string(), "t".to_string(),
            MessageType::Request, Value::Null
        )).await.unwrap();
        
        bus.send("target", Message::new(
            "s".to_string(), "t".to_string(),
            MessageType::Response, Value::Null
        )).await.unwrap();
        
        bus.send("target", Message::new(
            "s".to_string(), "t".to_string(),
            MessageType::Request, Value::Null
        )).await.unwrap();
        
        let stats = bus.get_stats().await;
        assert_eq!(stats.total_messages, 3);
        assert_eq!(stats.message_type_counts.get("Request"), Some(&2));
        assert_eq!(stats.message_type_counts.get("Response"), Some(&1));
    }

    #[tokio::test]
    async fn test_resource_limits() {
        let limits = interface::ResourceLimits::default();
        
        assert_eq!(limits.max_memory_bytes, 100 * 1024 * 1024);
        assert_eq!(limits.max_cpu_time_ms, 5000);
        assert_eq!(limits.max_file_handles, 10);
        assert!(!limits.network_access);
    }

    #[tokio::test]
    async fn test_module_format_detection() {
        use std::path::Path;
        
        assert_eq!(
            ModuleFormat::from_extension(Path::new("module.wasm")),
            Some(ModuleFormat::Wasm)
        );
        
        assert_eq!(
            ModuleFormat::from_extension(Path::new("module.dll")),
            Some(ModuleFormat::Native)
        );
        
        assert_eq!(
            ModuleFormat::from_extension(Path::new("module.so")),
            Some(ModuleFormat::Native)
        );
        
        assert_eq!(
            ModuleFormat::from_extension(Path::new("module.dylib")),
            Some(ModuleFormat::Native)
        );
        
        assert_eq!(
            ModuleFormat::from_extension(Path::new("module.txt")),
            None
        );
    }

    #[test]
    fn test_module_loader_creation() {
        // This test doesn't need async
        let loader = ModuleLoader::new();
        assert!(loader.is_ok());
    }
}