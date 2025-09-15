// Traffic Controller - Manages request routing during hot-reload

use super::{HotReloadError, HotReloadResult};
use crate::modules::Message;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// State of traffic for a module
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrafficState {
    /// Normal operation - all traffic flows to module
    Normal,
    /// Draining - no new requests, existing ones complete
    Draining,
    /// Buffering - all traffic is buffered temporarily
    Buffering,
    /// Paused - no traffic accepted
    Paused,
}

/// Information about a module's traffic
#[derive(Debug, Clone)]
struct ModuleTraffic {
    state: TrafficState,
    active_requests: usize,
    buffered_messages: VecDeque<Message>,
    max_buffer_size: usize,
}

impl ModuleTraffic {
    fn new() -> Self {
        Self {
            state: TrafficState::Normal,
            active_requests: 0,
            buffered_messages: VecDeque::new(),
            max_buffer_size: 10000,
        }
    }
}

/// Controls traffic flow to modules during reload
pub struct TrafficController {
    modules: Arc<RwLock<HashMap<String, ModuleTraffic>>>,
    drain_timeout: Duration,
}

impl TrafficController {
    pub fn new(drain_timeout: Duration) -> Self {
        Self {
            modules: Arc::new(RwLock::new(HashMap::new())),
            drain_timeout,
        }
    }

    /// Start draining traffic from a module
    pub async fn start_draining(&self, module_id: &str) -> HotReloadResult<()> {
        let mut modules = self.modules.write().await;
        let traffic = modules.entry(module_id.to_string()).or_insert_with(ModuleTraffic::new);
        
        if traffic.state != TrafficState::Normal {
            return Err(HotReloadError::VerificationFailed(
                format!("Module {} is not in normal state", module_id),
            ));
        }
        
        traffic.state = TrafficState::Draining;
        info!("Started draining traffic for module: {}", module_id);
        
        Ok(())
    }

    /// Check if a module has been fully drained
    pub async fn is_drained(&self, module_id: &str) -> bool {
        let modules = self.modules.read().await;
        
        if let Some(traffic) = modules.get(module_id) {
            traffic.state == TrafficState::Draining && traffic.active_requests == 0
        } else {
            true // No traffic tracked means drained
        }
    }

    /// Cancel draining and return to normal
    pub async fn cancel_draining(&self, module_id: &str) {
        let mut modules = self.modules.write().await;
        
        if let Some(traffic) = modules.get_mut(module_id) {
            if traffic.state == TrafficState::Draining {
                traffic.state = TrafficState::Normal;
                info!("Cancelled draining for module: {}", module_id);
            }
        }
    }

    /// Start buffering messages for a module
    pub async fn start_buffering(&self, module_id: &str) {
        let mut modules = self.modules.write().await;
        let traffic = modules.entry(module_id.to_string()).or_insert_with(ModuleTraffic::new);
        
        traffic.state = TrafficState::Buffering;
        debug!("Started buffering messages for module: {}", module_id);
    }

    /// Resume normal traffic and deliver buffered messages
    pub async fn resume_traffic(&self, module_id: &str) -> HotReloadResult<usize> {
        let mut modules = self.modules.write().await;
        
        if let Some(traffic) = modules.get_mut(module_id) {
            let message_count = traffic.buffered_messages.len();
            
            // Return to normal state
            traffic.state = TrafficState::Normal;
            
            // Messages will be delivered by the caller
            info!(
                "Resumed traffic for module: {} ({} buffered messages)",
                module_id, message_count
            );
            
            Ok(message_count)
        } else {
            Ok(0)
        }
    }

    /// Get buffered messages for delivery
    pub async fn get_buffered_messages(&self, module_id: &str) -> Vec<Message> {
        let mut modules = self.modules.write().await;
        
        if let Some(traffic) = modules.get_mut(module_id) {
            traffic.buffered_messages.drain(..).collect()
        } else {
            Vec::new()
        }
    }

    /// Check if a module can accept new traffic
    pub async fn can_accept_traffic(&self, module_id: &str) -> bool {
        let modules = self.modules.read().await;
        
        if let Some(traffic) = modules.get(module_id) {
            matches!(traffic.state, TrafficState::Normal)
        } else {
            true // Default to accepting if not tracked
        }
    }

    /// Route a message to a module (returns true if buffered)
    pub async fn route_message(&self, module_id: &str, message: Message) -> HotReloadResult<bool> {
        let mut modules = self.modules.write().await;
        let traffic = modules.entry(module_id.to_string()).or_insert_with(ModuleTraffic::new);
        
        match traffic.state {
            TrafficState::Normal => Ok(false), // Let it through
            TrafficState::Draining => {
                // Don't accept new requests during drain
                Err(HotReloadError::DrainTimeout)
            }
            TrafficState::Buffering => {
                // Buffer the message
                if traffic.buffered_messages.len() >= traffic.max_buffer_size {
                    return Err(HotReloadError::MemoryLimitExceeded);
                }
                
                traffic.buffered_messages.push_back(message);
                Ok(true)
            }
            TrafficState::Paused => {
                Err(HotReloadError::VerificationFailed(
                    "Module traffic is paused".to_string(),
                ))
            }
        }
    }

    /// Track an active request
    pub async fn track_request(&self, module_id: &str) {
        let mut modules = self.modules.write().await;
        let traffic = modules.entry(module_id.to_string()).or_insert_with(ModuleTraffic::new);
        
        traffic.active_requests += 1;
        debug!(
            "Active requests for module {}: {}",
            module_id, traffic.active_requests
        );
    }

    /// Mark a request as completed
    pub async fn complete_request(&self, module_id: &str) {
        let mut modules = self.modules.write().await;
        
        if let Some(traffic) = modules.get_mut(module_id) {
            if traffic.active_requests > 0 {
                traffic.active_requests -= 1;
                debug!(
                    "Active requests for module {}: {}",
                    module_id, traffic.active_requests
                );
            }
        }
    }

    /// Get the current traffic state for a module
    pub async fn get_traffic_state(&self, module_id: &str) -> TrafficState {
        let modules = self.modules.read().await;
        
        modules
            .get(module_id)
            .map(|t| t.state)
            .unwrap_or(TrafficState::Normal)
    }

    /// Get statistics about module traffic
    pub async fn get_traffic_stats(&self, module_id: &str) -> Option<(TrafficState, usize, usize)> {
        let modules = self.modules.read().await;
        
        modules.get(module_id).map(|t| {
            (t.state, t.active_requests, t.buffered_messages.len())
        })
    }

    /// Pause all traffic to a module
    pub async fn pause_traffic(&self, module_id: &str) {
        let mut modules = self.modules.write().await;
        let traffic = modules.entry(module_id.to_string()).or_insert_with(ModuleTraffic::new);
        
        traffic.state = TrafficState::Paused;
        warn!("Paused all traffic to module: {}", module_id);
    }

    /// Clear all traffic information for a module
    pub async fn clear_module(&self, module_id: &str) {
        let mut modules = self.modules.write().await;
        modules.remove(module_id);
        
        debug!("Cleared traffic info for module: {}", module_id);
    }

    /// Force drain a module (ignore active requests)
    pub async fn force_drain(&self, module_id: &str) -> usize {
        let mut modules = self.modules.write().await;
        
        if let Some(traffic) = modules.get_mut(module_id) {
            let active = traffic.active_requests;
            traffic.active_requests = 0;
            traffic.state = TrafficState::Normal;
            
            warn!(
                "Force drained module {} with {} active requests",
                module_id, active
            );
            
            active
        } else {
            0
        }
    }
}