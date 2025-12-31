---
sidebar_position: 3
title: Testing
description: Test your EventFlux streaming applications
---

# Testing

This guide covers testing strategies for EventFlux applications, from unit tests to integration tests.

## AppRunner Test Helper

The `AppRunner` is the primary testing utility for EventFlux applications:

```rust
use eventflux::testing::AppRunner;

#[test]
fn test_basic_filter() {
    let app = r#"
        DEFINE STREAM Input (value INT);

        SELECT value
        FROM Input
        WHERE value > 10
        INSERT INTO Output;
    "#;

    let runner = AppRunner::new(app, "Output");

    // Send test events
    runner.send("Input", vec![
        event![5],   // Should be filtered
        event![15],  // Should pass
        event![8],   // Should be filtered
        event![20],  // Should pass
    ]);

    // Get results and verify
    let results = runner.shutdown();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].get::<i32>(0), Some(15));
    assert_eq!(results[1].get::<i32>(0), Some(20));
}
```

## Testing Patterns

### Testing Filters

```rust
#[test]
fn test_temperature_filter() {
    let app = r#"
        DEFINE STREAM Sensors (sensor_id STRING, temp DOUBLE);

        SELECT sensor_id, temp
        FROM Sensors
        WHERE temp > 100.0
        INSERT INTO Alerts;
    "#;

    let runner = AppRunner::new(app, "Alerts");

    runner.send("Sensors", vec![
        event!["s1", 95.0],
        event!["s2", 105.0],
        event!["s3", 98.0],
        event!["s4", 110.0],
    ]);

    let results = runner.shutdown();
    assert_eq!(results.len(), 2);

    // Verify specific values
    assert_eq!(results[0].get::<String>(0), Some("s2".to_string()));
    assert_eq!(results[1].get::<String>(0), Some("s4".to_string()));
}
```

### Testing Aggregations

```rust
#[test]
fn test_sum_aggregation() {
    let app = r#"
        DEFINE STREAM Sales (product STRING, amount DOUBLE);

        SELECT product, SUM(amount) AS total
        FROM Sales
        WINDOW LENGTHBATCH(3)
        GROUP BY product
        INSERT INTO Totals;
    "#;

    let runner = AppRunner::new(app, "Totals");

    runner.send("Sales", vec![
        event!["A", 10.0],
        event!["A", 20.0],
        event!["A", 30.0],  // Batch completes here
    ]);

    let results = runner.shutdown();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].get::<f64>(1), Some(60.0));
}
```

### Testing Windows

```rust
#[test]
fn test_length_window() {
    let app = r#"
        DEFINE STREAM Numbers (value INT);

        SELECT AVG(value) AS avg_value
        FROM Numbers
        WINDOW LENGTH(3)
        INSERT INTO Averages;
    "#;

    let runner = AppRunner::new(app, "Averages");

    // Window fills up
    runner.send("Numbers", vec![event![10]]);  // avg = 10
    runner.send("Numbers", vec![event![20]]);  // avg = 15
    runner.send("Numbers", vec![event![30]]);  // avg = 20

    // Window slides
    runner.send("Numbers", vec![event![40]]);  // [20, 30, 40] avg = 30

    let results = runner.shutdown();

    // Verify sliding window behavior
    assert!(results.len() >= 4);
}
```

### Testing Joins

```rust
#[test]
fn test_stream_join() {
    let app = r#"
        DEFINE STREAM Orders (order_id STRING, customer_id STRING);
        DEFINE STREAM Payments (order_id STRING, amount DOUBLE);

        SELECT o.order_id, o.customer_id, p.amount
        FROM Orders AS o
        WINDOW LENGTHBATCH(2)
        JOIN Payments AS p
          ON o.order_id = p.order_id
        INSERT INTO Matched;
    "#;

    let runner = AppRunner::new(app, "Matched");

    runner.send("Orders", vec![event!["O1", "C1"]]);
    runner.send("Payments", vec![event!["O1", 100.0]]);
    runner.send("Orders", vec![event!["O2", "C2"]]);
    runner.send("Payments", vec![event!["O2", 200.0]]);

    let results = runner.shutdown();
    // Verify join results
    assert!(!results.is_empty());
}
```

### Testing Patterns

```rust
#[test]
fn test_sequence_pattern() {
    let app = r#"
        DEFINE STREAM Events (event_type STRING, value INT);

        FROM Events
        MATCH (e1=Event -> e2=Event)
        WITHIN 1 MINUTE
        FILTER e1.event_type = 'START' AND e2.event_type = 'END'
        SELECT e1.value AS start_value, e2.value AS end_value
        INSERT INTO Sequences;
    "#;

    let runner = AppRunner::new(app, "Sequences");

    runner.send("Events", vec![
        event!["START", 1],
        event!["END", 2],
    ]);

    let results = runner.shutdown();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].get::<i32>(0), Some(1));
    assert_eq!(results[0].get::<i32>(1), Some(2));
}
```

## Advanced Testing

### Testing Multiple Outputs

```rust
#[test]
fn test_multiple_outputs() {
    let app = r#"
        DEFINE STREAM Input (value INT);

        SELECT value FROM Input WHERE value > 50 INSERT INTO High;
        SELECT value FROM Input WHERE value <= 50 INSERT INTO Low;
    "#;

    // Test high output
    let high_runner = AppRunner::new(app, "High");
    high_runner.send("Input", vec![event![30], event![70], event![40], event![80]]);
    let high_results = high_runner.shutdown();
    assert_eq!(high_results.len(), 2);

    // Test low output
    let low_runner = AppRunner::new(app, "Low");
    low_runner.send("Input", vec![event![30], event![70], event![40], event![80]]);
    let low_results = low_runner.shutdown();
    assert_eq!(low_results.len(), 2);
}
```

### Testing with Time Advancement

```rust
#[test]
fn test_time_window() {
    let app = r#"
        DEFINE STREAM Events (ts LONG, value INT);

        SELECT COUNT(*) AS count
        FROM Events
        WINDOW TUMBLING(5 sec)
        INSERT INTO Counts;
    "#;

    let runner = AppRunner::new(app, "Counts");

    // Events in first window
    runner.send_with_time("Events", vec![
        event![1000i64, 1],
        event![2000i64, 2],
        event![3000i64, 3],
    ], 1000);

    // Advance time to trigger window
    runner.advance_time(6000);

    // Events in second window
    runner.send_with_time("Events", vec![
        event![6000i64, 4],
        event![7000i64, 5],
    ], 6000);

    runner.advance_time(11000);

    let results = runner.shutdown();
    assert_eq!(results[0].get::<i64>(0), Some(3)); // First window
    assert_eq!(results[1].get::<i64>(0), Some(2)); // Second window
}
```

### Testing Error Conditions

```rust
#[test]
fn test_invalid_query() {
    let manager = EventFluxManager::new();

    let result = manager.create_runtime("INVALID SQL QUERY");
    assert!(result.is_err());

    if let Err(EventFluxError::ParseError(msg)) = result {
        assert!(msg.contains("syntax"));
    }
}

#[test]
fn test_stream_not_found() {
    let app = r#"
        DEFINE STREAM Input (value INT);
        SELECT value FROM Input INSERT INTO Output;
    "#;

    let manager = EventFluxManager::new();
    let runtime = manager.create_runtime(app).unwrap();

    let result = runtime.send("NonExistent", event![42]);
    assert!(matches!(result, Err(EventFluxError::StreamNotFound(_))));
}
```

## Test Organization

### Recommended Structure

```
tests/
├── unit/
│   ├── filter_tests.rs
│   ├── window_tests.rs
│   ├── join_tests.rs
│   └── pattern_tests.rs
├── integration/
│   ├── end_to_end_tests.rs
│   └── performance_tests.rs
└── common/
    └── mod.rs  # Shared test utilities
```

### Shared Test Utilities

```rust
// tests/common/mod.rs
use eventflux::testing::AppRunner;

pub fn create_sensor_app() -> &'static str {
    r#"
        DEFINE STREAM Sensors (sensor_id STRING, value DOUBLE);

        SELECT sensor_id, value
        FROM Sensors
        WHERE value > 100.0
        INSERT INTO Alerts;
    "#
}

pub fn send_sensor_events(runner: &AppRunner, count: usize) {
    for i in 0..count {
        runner.send("Sensors", vec![
            event![format!("s{}", i), (i as f64) * 10.0],
        ]);
    }
}
```

## Property-Based Testing

Using `proptest` for property-based testing:

```rust
use proptest::prelude::*;
use eventflux::testing::AppRunner;

proptest! {
    #[test]
    fn filter_preserves_order(values in prop::collection::vec(any::<i32>(), 0..100)) {
        let app = r#"
            DEFINE STREAM Input (value INT);
            SELECT value FROM Input WHERE value > 0 INSERT INTO Output;
        "#;

        let runner = AppRunner::new(app, "Output");

        for v in &values {
            runner.send("Input", vec![event![*v]]);
        }

        let results = runner.shutdown();
        let result_values: Vec<i32> = results
            .iter()
            .filter_map(|e| e.get::<i32>(0))
            .collect();

        // Verify order is preserved
        let expected: Vec<i32> = values.into_iter().filter(|&v| v > 0).collect();
        prop_assert_eq!(result_values, expected);
    }
}
```

## Best Practices

:::tip Testing Guidelines

1. **Test one thing at a time** - Each test should verify a single behavior
2. **Use descriptive names** - Test names should describe the scenario
3. **Test edge cases** - Empty inputs, boundary values, etc.
4. **Keep tests fast** - Use small data sets
5. **Make tests deterministic** - Avoid time-dependent tests when possible

:::

:::caution Common Pitfalls

- **Race conditions** - Use single-threaded config for determinism
- **Timing issues** - Use explicit time advancement
- **Flaky tests** - Ensure proper synchronization
- **Over-mocking** - Test real behavior when practical

:::

## Next Steps

- [Configuration](/docs/rust-api/configuration) - Configure test environments
- [SQL Reference](/docs/sql-reference/queries) - Query syntax for tests
- [Architecture](/docs/architecture/overview) - Understand internals for debugging
