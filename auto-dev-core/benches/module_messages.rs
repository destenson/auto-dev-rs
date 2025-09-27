use auto_dev_core::modules::messages::{Message, MessageBus, MessageType};
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use serde_json::Value;

fn bench_message_passing(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("message_send_broadcast", |b| {
        let bus = MessageBus::new();
        let msg = Message::new(
            "sender".to_string(),
            "*".to_string(),
            MessageType::Broadcast,
            Value::Null,
        );

        b.to_async(&runtime).iter(|| async {
            bus.broadcast(black_box(msg.clone())).await.unwrap();
        });
    });

    c.bench_function("message_send_targeted", |b| {
        let bus = MessageBus::new();
        let msg = Message::new(
            "sender".to_string(),
            "target".to_string(),
            MessageType::Request,
            Value::Null,
        );

        b.to_async(&runtime).iter(|| async {
            bus.send(black_box("target"), black_box(msg.clone())).await.unwrap();
        });
    });

    c.bench_function("message_roundtrip", |b| {
        let bus = MessageBus::new();

        b.to_async(&runtime).iter(|| async {
            let mut receiver = bus.subscribe_broadcast();

            let msg = Message::new(
                "sender".to_string(),
                "*".to_string(),
                MessageType::Broadcast,
                Value::Null,
            );

            bus.broadcast(msg).await.unwrap();
            let _ = receiver.recv().await;
        });
    });

    c.bench_function("message_throughput_1000", |b| {
        let bus = MessageBus::new();

        b.to_async(&runtime).iter(|| async {
            for i in 0..1000 {
                let msg = Message::new(
                    "sender".to_string(),
                    "*".to_string(),
                    MessageType::Broadcast,
                    serde_json::json!({"index": i}),
                );
                bus.broadcast(msg).await.unwrap();
            }
        });
    });
}

criterion_group!(benches, bench_message_passing);
criterion_main!(benches);
