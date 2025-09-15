use auto_dev_core::modules::{
    hot_reload::{HotReloadConfig, ReloadCoordinator, StateManager, TrafficController},
    interface::ModuleState,
    ModuleVersion,
};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::time::Duration;
use tokio::runtime::Runtime;

fn bench_state_snapshot(c: &mut Criterion) {
    let runtime = Runtime::new().unwrap();

    c.bench_function("state_snapshot_create", |b| {
        let state_manager = StateManager::new();
        let mut state = ModuleState::new(ModuleVersion::new(1, 0, 0));
        
        // Add some data to state
        for i in 0..100 {
            state.set(
                format!("field_{}", i),
                serde_json::json!({
                    "value": i,
                    "data": vec![i; 100],
                }),
            );
        }

        b.to_async(&runtime).iter(|| async {
            state_manager
                .create_snapshot(black_box("test_module"), black_box(&state))
                .await
                .unwrap();
        });
    });

    c.bench_function("state_diff_snapshot", |b| {
        let state_manager = StateManager::new();
        let mut state = ModuleState::new(ModuleVersion::new(1, 0, 0));
        
        for i in 0..100 {
            state.set(format!("field_{}", i), serde_json::json!(i));
        }

        b.to_async(&runtime).iter(|| async {
            let snapshot = state_manager
                .create_snapshot("test_module", &state)
                .await
                .unwrap();
            
            // Modify some fields
            let mut new_state = state.clone();
            new_state.set("field_0".to_string(), serde_json::json!(999));
            
            state_manager
                .create_diff_snapshot(
                    black_box("test_module"),
                    black_box(&new_state),
                    Some(&snapshot),
                )
                .await
                .unwrap();
        });
    });
}

fn bench_traffic_control(c: &mut Criterion) {
    let runtime = Runtime::new().unwrap();

    c.bench_function("traffic_draining", |b| {
        let controller = TrafficController::new(Duration::from_secs(5));

        b.to_async(&runtime).iter(|| async {
            controller.start_draining(black_box("test_module")).await.unwrap();
            
            // Simulate some active requests
            for _ in 0..10 {
                controller.track_request("test_module").await;
            }
            
            // Complete all requests
            for _ in 0..10 {
                controller.complete_request("test_module").await;
            }
            
            assert!(controller.is_drained("test_module").await);
            
            // Clean up for next iteration
            controller.clear_module("test_module").await;
        });
    });

    c.bench_function("message_buffering", |b| {
        use auto_dev_core::modules::messages::{Message, MessageType};
        let controller = TrafficController::new(Duration::from_secs(5));

        b.to_async(&runtime).iter(|| async {
            controller.start_buffering(black_box("test_module")).await;

            // Buffer 100 messages
            for i in 0..100 {
                let msg = Message::new(
                    "sender".to_string(),
                    "test_module".to_string(),
                    MessageType::Request,
                    serde_json::json!({"index": i}),
                );
                controller.route_message("test_module", msg).await.unwrap();
            }

            // Resume and get messages
            let count = controller.resume_traffic("test_module").await.unwrap();
            assert_eq!(count, 100);

            let messages = controller.get_buffered_messages("test_module").await;
            assert_eq!(messages.len(), 100);

            // Clean up
            controller.clear_module("test_module").await;
        });
    });
}

fn bench_reload_phases(c: &mut Criterion) {
    let runtime = Runtime::new().unwrap();

    c.bench_function("reload_coordinator_phase_transition", |b| {
        let config = HotReloadConfig {
            drain_timeout: Duration::from_millis(10),
            reload_timeout: Duration::from_secs(1),
            verification_delay: Duration::from_millis(1),
            ..Default::default()
        };
        
        let coordinator = ReloadCoordinator::new(config);

        b.to_async(&runtime).iter(|| async {
            // Test phase tracking performance
            for i in 0..10 {
                let module_id = format!("module_{}", i);
                coordinator.is_reloading(black_box(&module_id)).await;
                coordinator.get_reload_phase(black_box(&module_id)).await;
            }
        });
    });
}

fn bench_state_migration(c: &mut Criterion) {
    use auto_dev_core::modules::hot_reload::{
        FieldMapping, MigrationEngine, MigrationRule, StateVersion,
    };
    let runtime = Runtime::new().unwrap();

    c.bench_function("state_migration_simple", |b| {
        let engine = MigrationEngine::new();

        b.to_async(&runtime).iter(|| async {
            let mut state = ModuleState::new(ModuleVersion::new(1, 0, 0));
            
            // Add fields to migrate
            for i in 0..50 {
                state.set(format!("field_{}", i), serde_json::json!(i));
            }

            let from_version = StateVersion::new(1, 0, 0);
            let to_version = StateVersion::new(1, 0, 1);

            // Simple compatible migration (no rules needed)
            engine
                .migrate_state(
                    black_box(state),
                    black_box(from_version),
                    black_box(to_version),
                )
                .await
                .unwrap();
        });
    });

    c.bench_function("state_migration_with_rules", |b| {
        let engine = MigrationEngine::new();
        let runtime_clone = Runtime::new().unwrap();

        runtime_clone.block_on(async {
            // Register migration rules
            let rule = MigrationRule {
                from_version: StateVersion::new(1, 0, 0),
                to_version: StateVersion::new(2, 0, 0),
                field_mappings: std::collections::HashMap::from([
                    ("old_field".to_string(), FieldMapping::Rename("new_field".to_string())),
                ]),
                new_fields: std::collections::HashMap::from([
                    ("added".to_string(), serde_json::json!("default")),
                ]),
                removed_fields: vec!["deprecated".to_string()],
                custom_transform: None,
            };
            engine.register_rule(rule).await;
        });

        b.to_async(&runtime).iter(|| async {
            let mut state = ModuleState::new(ModuleVersion::new(1, 0, 0));
            state.set("old_field".to_string(), serde_json::json!("value"));
            state.set("deprecated".to_string(), serde_json::json!("remove_me"));

            engine
                .migrate_state(
                    black_box(state),
                    black_box(StateVersion::new(1, 0, 0)),
                    black_box(StateVersion::new(2, 0, 0)),
                )
                .await
                .unwrap();
        });
    });
}

criterion_group!(
    benches,
    bench_state_snapshot,
    bench_traffic_control,
    bench_reload_phases,
    bench_state_migration
);
criterion_main!(benches);