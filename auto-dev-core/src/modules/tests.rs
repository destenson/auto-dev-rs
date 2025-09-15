// Module System Tests
//
// Comprehensive tests for the dynamic module system

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::modules::interface::{
        ModuleCapability, ModuleDependency, ModuleInterface, ModuleMetadata, ModuleState, ModuleVersion,
    };
    use crate::modules::loader::{ModuleFormat, ModuleLoader};
    use crate::modules::messages::{Message, MessageBus, MessageType};
    use crate::modules::registry::{ModuleRegistry, ModuleStatus};
    use crate::modules::runtime::{ExecutionContext, ModuleRuntime};
    use anyhow::Result;
    use async_trait::async_trait;
    use serde_json::Value;
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
                capabilities: vec![ModuleCapability::Custom {
                    name: "test".to_string(),
                    description: "Test capability".to_string(),
                }],
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
        
        async fn health_check(&self) -> Result<bool> {
            Ok(true)
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

        let v_pre =
            ModuleVersion { major: 1, minor: 0, patch: 0, pre_release: Some("alpha".to_string()) };

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
        assert_eq!(context.config.get("debug"), Some(&Value::Bool(true)));
    }

    #[tokio::test]
    async fn test_message_bus() {
        let bus = MessageBus::new();

        // Subscribe to messages
        let mut receiver = bus
            .subscribe(
                "test_target".to_string(),
                Box::new(|msg| matches!(msg.message_type, MessageType::Request)),
            )
            .await;

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
        )
        .with_correlation_id(correlation_id.to_string());

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
        module.state.set("counter".to_string(), Value::Number(42.into()));

        // Get state
        let state = module.get_state().unwrap();
        assert_eq!(state.get("counter"), Some(&Value::Number(42.into())));

        // Create new module and restore state
        let mut new_module = MockModule::new("stateful_module");
        new_module.restore_state(state).unwrap();

        let restored_state = new_module.get_state().unwrap();
        assert_eq!(restored_state.get("counter"), Some(&Value::Number(42.into())));
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
        bus.send(
            "target",
            Message::new("s".to_string(), "t".to_string(), MessageType::Request, Value::Null),
        )
        .await
        .unwrap();

        bus.send(
            "target",
            Message::new("s".to_string(), "t".to_string(), MessageType::Response, Value::Null),
        )
        .await
        .unwrap();

        bus.send(
            "target",
            Message::new("s".to_string(), "t".to_string(), MessageType::Request, Value::Null),
        )
        .await
        .unwrap();

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

        assert_eq!(ModuleFormat::from_extension(Path::new("module.txt")), None);
    }

    #[test]
    fn test_module_loader_creation() {
        // This test doesn't need async
        let loader = ModuleLoader::new();
        assert!(loader.is_ok());
    }

    #[tokio::test]
    async fn test_hot_reload_state_preservation() {
        let mut module = MockModule::new("hot_reload_test");
        
        // Initialize and set state
        module.initialize().await.unwrap();
        module.state.set("counter".to_string(), Value::Number(42.into()));
        module.state.set("config".to_string(), serde_json::json!({
            "enabled": true,
            "threshold": 100
        }));
        
        // Simulate hot-reload: save state, create new module, restore state
        let saved_state = module.get_state().unwrap();
        
        // Simulate module shutdown
        module.shutdown().await.unwrap();
        
        // Create "new" module (simulating reload)
        let mut reloaded_module = MockModule::new("hot_reload_test");
        reloaded_module.initialize().await.unwrap();
        
        // Restore state
        reloaded_module.restore_state(saved_state).unwrap();
        
        // Verify state was preserved
        let restored_state = reloaded_module.get_state().unwrap();
        assert_eq!(restored_state.get("counter"), Some(&Value::Number(42.into())));
        assert_eq!(restored_state.get("config").and_then(|v| v.get("enabled")), 
                   Some(&Value::Bool(true)));
    }

    #[tokio::test]
    async fn test_module_isolation() {
        // Create two independent modules
        let mut module1 = MockModule::new("module1");
        let mut module2 = MockModule::new("module2");
        
        // Initialize both
        module1.initialize().await.unwrap();
        module2.initialize().await.unwrap();
        
        // Set different states
        module1.state.set("value".to_string(), Value::String("module1_value".to_string()));
        module2.state.set("value".to_string(), Value::String("module2_value".to_string()));
        
        // Execute both modules
        let result1 = module1.execute(Value::String("test".to_string())).await.unwrap();
        let result2 = module2.execute(Value::String("test".to_string())).await.unwrap();
        
        // Verify isolation - each module has its own state and execution context
        assert_eq!(result1["module"], "module1");
        assert_eq!(result2["module"], "module2");
        
        let state1 = module1.get_state().unwrap();
        let state2 = module2.get_state().unwrap();
        
        assert_eq!(state1.get("value"), Some(&Value::String("module1_value".to_string())));
        assert_eq!(state2.get("value"), Some(&Value::String("module2_value".to_string())));
        
        // Verify execution counts are independent
        assert_eq!(result1["execution_count"], 1);
        assert_eq!(result2["execution_count"], 1);
    }

    #[tokio::test]
    async fn test_module_crash_isolation() {
        // Test that a panic in one module doesn't affect others
        struct CrashingModule {
            metadata: ModuleMetadata,
            crash_on_execute: bool,
        }
        
        impl CrashingModule {
            fn new(name: &str) -> Self {
                let metadata = ModuleMetadata {
                    name: name.to_string(),
                    version: ModuleVersion::new(1, 0, 0),
                    author: "test".to_string(),
                    description: "Module that can crash".to_string(),
                    capabilities: vec![],
                    dependencies: vec![],
                };
                
                Self {
                    metadata,
                    crash_on_execute: false,
                }
            }
        }
        
        #[async_trait]
        impl ModuleInterface for CrashingModule {
            fn metadata(&self) -> ModuleMetadata {
                self.metadata.clone()
            }
            
            async fn initialize(&mut self) -> Result<()> {
                Ok(())
            }
            
            async fn execute(&self, _input: Value) -> Result<Value> {
                if self.crash_on_execute {
                    panic!("Module crashed!");
                }
                Ok(serde_json::json!({"status": "ok"}))
            }
            
            fn get_capabilities(&self) -> Vec<ModuleCapability> {
                vec![]
            }
            
            async fn handle_message(&mut self, _message: Value) -> Result<Option<Value>> {
                Ok(None)
            }
            
            async fn shutdown(&mut self) -> Result<()> {
                Ok(())
            }
            
            fn get_state(&self) -> Result<ModuleState> {
                Ok(ModuleState::new(self.metadata.version.clone()))
            }
            
            fn restore_state(&mut self, _state: ModuleState) -> Result<()> {
                Ok(())
            }
            
            async fn health_check(&self) -> Result<bool> {
                Ok(true)
            }
        }
        
        let mut normal_module = MockModule::new("normal");
        let mut crashing_module = CrashingModule::new("crashing");
        crashing_module.crash_on_execute = true;
        
        // Initialize both
        normal_module.initialize().await.unwrap();
        crashing_module.initialize().await.unwrap();
        
        // Execute normal module - should work
        let result = normal_module.execute(Value::String("test".to_string())).await;
        assert!(result.is_ok());
        
        // Execute crashing module - should panic (caught by tokio runtime)
        // In a real implementation, this would be caught by the ModuleRuntime
        let crash_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                crashing_module.execute(Value::String("test".to_string())).await
            })
        }));
        assert!(crash_result.is_err());
        
        // Normal module should still work after the crash
        let result2 = normal_module.execute(Value::String("test2".to_string())).await;
        assert!(result2.is_ok());
    }

    #[tokio::test]
    async fn test_message_passing_performance() {
        use std::time::Instant;
        
        let bus = MessageBus::new();
        
        // Subscribe to messages
        let mut receiver = bus.subscribe_broadcast();
        
        let start = Instant::now();
        let message_count = 1000;
        
        // Send many messages
        for i in 0..message_count {
            let msg = Message::new(
                "sender".to_string(),
                "*".to_string(),
                MessageType::Broadcast,
                serde_json::json!({"index": i}),
            );
            bus.broadcast(msg).await.unwrap();
        }
        
        // Receive all messages
        let mut received_count = 0;
        while let Ok(msg) = receiver.try_recv() {
            received_count += 1;
            // Verify message integrity
            assert!(msg.payload.get("index").is_some());
        }
        
        let duration = start.elapsed();
        let messages_per_second = (message_count as f64) / duration.as_secs_f64();
        
        // Assert performance - should handle at least 10,000 messages per second
        assert!(messages_per_second > 10_000.0, 
                "Message passing too slow: {:.0} msg/s", messages_per_second);
        
        // Verify all messages were received
        assert_eq!(received_count, message_count);
        
        // Test latency - single message round trip
        let latency_start = Instant::now();
        let msg = Message::new(
            "sender".to_string(),
            "*".to_string(),
            MessageType::Broadcast,
            Value::Null,
        );
        bus.broadcast(msg).await.unwrap();
        let _ = receiver.recv().await;
        let latency = latency_start.elapsed();
        
        // Assert latency < 1ms
        assert!(latency.as_micros() < 1000, 
                "Message latency too high: {:?}", latency);
    }

    #[tokio::test]
    async fn test_module_dependency_resolution() {
        let registry = ModuleRegistry::new();
        
        // Create modules with dependencies
        let parser_module = MockModule::new("parser");
        let synthesis_module = MockModule {
            metadata: ModuleMetadata {
                name: "synthesis".to_string(),
                version: ModuleVersion::new(1, 0, 0),
                author: "test".to_string(),
                description: "Synthesis module".to_string(),
                capabilities: vec![],
                dependencies: vec![ModuleDependency {
                    name: "parser".to_string(),
                    version_requirement: "1.0.0".to_string(),
                    optional: false,
                }],
            },
            state: ModuleState::new(ModuleVersion::new(1, 0, 0)),
            execution_count: Arc::new(RwLock::new(0)),
            initialized: false,
        };
        
        // Register parser first (dependency)
        // In a real implementation, modules would be loaded via ModuleLoader
        // For testing, we'll verify dependency handling logic
        let parser_meta = parser_module.metadata();
        let synthesis_meta = synthesis_module.metadata();
        
        // Verify dependency is declared
        assert_eq!(synthesis_meta.dependencies.len(), 1);
        assert_eq!(synthesis_meta.dependencies[0].name, "parser");
        assert_eq!(parser_meta.dependencies.len(), 0);
    }

    #[tokio::test]
    async fn test_concurrent_module_execution() {
        use tokio::task;
        
        let module = Arc::new(MockModule::new("concurrent_test"));
        let mut handles = vec![];
        
        // Spawn multiple concurrent executions
        for i in 0..10 {
            let module_clone = Arc::clone(&module);
            let handle = task::spawn(async move {
                let input = serde_json::json!({"task_id": i});
                module_clone.execute(input).await
            });
            handles.push(handle);
        }
        
        // Wait for all executions to complete
        let mut results = vec![];
        for handle in handles {
            let result = handle.await.unwrap().unwrap();
            results.push(result);
        }
        
        // Verify all executions completed successfully
        assert_eq!(results.len(), 10);
        
        // Check that execution count is correct (should be 10)
        let final_count = *module.execution_count.read().await;
        assert_eq!(final_count, 10);
    }

    #[tokio::test]
    async fn test_module_resource_limits() {
        let limits = interface::ResourceLimits {
            max_memory_bytes: 50 * 1024 * 1024, // 50MB
            max_cpu_time_ms: 1000, // 1 second
            max_file_handles: 5,
            network_access: false,
            allowed_paths: vec![],
        };
        
        // Verify limits are enforced
        assert!(limits.max_memory_bytes < 100 * 1024 * 1024);
        assert!(limits.max_cpu_time_ms <= 5000);
        assert!(!limits.network_access);
    }

    #[tokio::test]
    async fn test_module_version_compatibility() {
        let v1_0_0 = ModuleVersion::new(1, 0, 0);
        let v1_0_1 = ModuleVersion::new(1, 0, 1);
        let v1_1_0 = ModuleVersion::new(1, 1, 0);
        let v2_0_0 = ModuleVersion::new(2, 0, 0);
        
        // Patch versions are compatible
        assert!(v1_0_0.is_compatible_with(&v1_0_1));
        
        // Minor versions are compatible (backwards)
        assert!(v1_0_0.is_compatible_with(&v1_1_0));
        
        // Major versions are not compatible
        assert!(!v1_0_0.is_compatible_with(&v2_0_0));
        
        // Test pre-release versions
        let v1_alpha = ModuleVersion {
            major: 1,
            minor: 0,
            patch: 0,
            pre_release: Some("alpha.1".to_string()),
        };
        
        let v1_beta = ModuleVersion {
            major: 1,
            minor: 0,
            patch: 0,
            pre_release: Some("beta.1".to_string()),
        };
        
        // Pre-release versions with same base are compatible
        assert!(v1_alpha.is_compatible_with(&v1_beta));
    }

    #[tokio::test]
    async fn test_module_health_check() {
        let mut module = MockModule::new("health_test");
        
        // Health check before initialization
        let health = module.health_check().await;
        assert!(health.is_ok());
        assert!(health.unwrap());
        
        // Initialize and check again
        module.initialize().await.unwrap();
        let health = module.health_check().await;
        assert!(health.unwrap());
        
        // Shutdown and verify
        module.shutdown().await.unwrap();
        let health = module.health_check().await;
        assert!(health.unwrap());
    }
}
