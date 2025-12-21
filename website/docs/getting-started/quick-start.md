---
sidebar_position: 2
title: Quick Start
description: Build your first streaming application with EventFlux
---

# Quick Start

This guide walks you through building your first streaming application with EventFlux in under 5 minutes.

## Your First Application

Let's create a simple temperature monitoring system that alerts when readings exceed a threshold.

### Step 1: Define Your Streams

First, define the input and output streams:

```sql
-- Input stream for sensor readings
DEFINE STREAM SensorReadings (
    sensor_id STRING,
    temperature DOUBLE,
    timestamp LONG
);
```

### Step 2: Write Your Query

Create a query that filters high temperature readings:

```sql
SELECT sensor_id, temperature, timestamp
FROM SensorReadings
WHERE temperature > 100.0
INSERT INTO HighTempAlerts;
```

### Step 3: Complete Application

Here's the complete Rust application:

```rust title="src/main.rs"
use eventflux::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the manager
    let manager = EventFluxManager::new();

    // Define the application
    let app = r#"
        DEFINE STREAM SensorReadings (
            sensor_id STRING,
            temperature DOUBLE,
            timestamp LONG
        );

        SELECT sensor_id, temperature, timestamp
        FROM SensorReadings
        WHERE temperature > 100.0
        INSERT INTO HighTempAlerts;
    "#;

    // Create and start the runtime
    let runtime = manager.create_runtime(app)?;
    runtime.start();

    // Register output callback
    runtime.on_output("HighTempAlerts", |event| {
        println!("ALERT: {:?}", event);
        Ok(())
    })?;

    // Send test events
    runtime.send("SensorReadings", event!["sensor-1", 95.0, 1000i64])?;
    runtime.send("SensorReadings", event!["sensor-2", 105.0, 1001i64])?;
    runtime.send("SensorReadings", event!["sensor-1", 110.0, 1002i64])?;

    // Output:
    // ALERT: ["sensor-2", 105.0, 1001]
    // ALERT: ["sensor-1", 110.0, 1002]

    Ok(())
}
```

## Adding Windows

Let's extend our application to compute rolling averages:

```rust
let app = r#"
    DEFINE STREAM SensorReadings (
        sensor_id STRING,
        temperature DOUBLE,
        timestamp LONG
    );

    -- Compute 5-minute rolling average per sensor
    SELECT sensor_id,
           AVG(temperature) AS avg_temp,
           MAX(temperature) AS max_temp,
           MIN(temperature) AS min_temp,
           COUNT(*) AS reading_count
    FROM SensorReadings
    WINDOW TUMBLING(5 min)
    GROUP BY sensor_id
    INSERT INTO SensorStats;

    -- Alert on high averages
    SELECT sensor_id, avg_temp
    FROM SensorStats
    WHERE avg_temp > 90.0
    INSERT INTO HighAvgAlerts;
"#;
```

## Working with Multiple Streams

Join data from multiple sources:

```rust
let app = r#"
    DEFINE STREAM Trades (
        symbol STRING,
        price DOUBLE,
        volume INT
    );

    DEFINE STREAM Quotes (
        symbol STRING,
        bid DOUBLE,
        ask DOUBLE
    );

    -- Join trades with quotes
    SELECT t.symbol,
           t.price AS trade_price,
           q.bid,
           q.ask,
           t.price - q.bid AS spread
    FROM Trades AS t
    WINDOW TUMBLING(1 sec)
    JOIN Quotes AS q
      ON t.symbol = q.symbol
    INSERT INTO TradeAnalysis;
"#;
```

## Pattern Detection

Detect event sequences:

```rust
let app = r#"
    DEFINE STREAM StockTrades (
        symbol STRING,
        price DOUBLE,
        timestamp LONG
    );

    -- Detect three consecutive price increases
    FROM StockTrades
    MATCH (e1=Trade -> e2=Trade -> e3=Trade)
    WITHIN 1 min
    FILTER e1.symbol = e2.symbol
       AND e2.symbol = e3.symbol
       AND e2.price > e1.price
       AND e3.price > e2.price
    SELECT e1.symbol,
           e1.price AS price1,
           e2.price AS price2,
           e3.price AS price3
    INSERT INTO PriceUptrend;
"#;
```

## Testing Your Application

Use the `AppRunner` for easy testing:

```rust
#[cfg(test)]
mod tests {
    use eventflux::testing::AppRunner;

    #[test]
    fn test_temperature_filter() {
        let app = r#"
            DEFINE STREAM Input (sensor_id STRING, temp DOUBLE);

            SELECT sensor_id, temp
            FROM Input
            WHERE temp > 100.0
            INSERT INTO Output;
        "#;

        let runner = AppRunner::new(app, "Output");

        // Send test events
        runner.send("Input", vec![
            event!["s1", 50.0],   // Below threshold
            event!["s2", 150.0],  // Above threshold
            event!["s1", 75.0],   // Below threshold
        ]);

        // Verify output
        let results = runner.shutdown();
        assert_eq!(results.len(), 1);
    }
}
```

## Running from CLI

You can also run EventFlux queries from the command line:

```bash
# Create a query file
cat > my_query.eventflux << 'EOF'
DEFINE STREAM Input (value INT);

SELECT value * 2 AS doubled
FROM Input
INSERT INTO Output;
EOF

# Run with the CLI
cargo run --bin run_eventflux my_query.eventflux
```

## Next Steps

Now that you've built your first application:

- **[SQL Reference](/docs/sql-reference/queries)** - Learn the complete query syntax
- **[Windows Guide](/docs/sql-reference/windows)** - Master time-based processing
- **[Pattern Matching](/docs/sql-reference/patterns)** - Detect complex event sequences
- **[Rust API](/docs/rust-api/getting-started)** - Deep dive into the API
