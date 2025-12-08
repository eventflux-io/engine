---
sidebar_position: 2
title: Windows
description: Time-based and count-based window operations in Eventflux
---

# Windows

Windows are fundamental to stream processing, allowing you to group events for aggregation and analysis. Eventflux supports **9 window types** to cover different streaming scenarios.

## Window Syntax

```sql
SELECT ...
FROM StreamName
WINDOW WindowType(parameters)
GROUP BY column
INSERT INTO Output;
```

## Window Types Overview

| Window | Type | Description | Use Case |
|--------|------|-------------|----------|
| **Tumbling** | Time | Fixed, non-overlapping | Hourly/daily reports |
| **Sliding** | Time | Overlapping with slide | Moving averages |
| **Session** | Time | Gap-based | User sessions |
| **Time** | Time | Continuous rolling | Real-time monitoring |
| **TimeBatch** | Time | Periodic batches | Scheduled snapshots |
| **ExternalTime** | Time | Event timestamp | Out-of-order events |
| **Length** | Count | Last N events | Recent history |
| **LengthBatch** | Count | N-event batches | Batch processing |
| **Delay** | Time | Delayed emission | Late arrival handling |

---

## Time-Based Windows

### Tumbling Window

Non-overlapping, fixed-size time windows. Events are assigned to exactly one window.

```sql
-- 5-minute tumbling windows
SELECT sensor_id,
       AVG(temperature) AS avg_temp,
       COUNT(*) AS reading_count
FROM SensorReadings
WINDOW TUMBLING(5 min)
GROUP BY sensor_id
INSERT INTO FiveMinuteStats;
```

**Time Units:** `sec`, `min`, `hour`, `day`

**Visual Representation:**
```
Events: ──●──●──●──●──●──●──●──●──●──●──●──●──●──▶
Windows: [────────][────────][────────][────────]
              W1        W2        W3        W4
```

### Sliding Window

Overlapping windows with configurable slide interval.

```sql
-- 10-second window, sliding every 2 seconds
SELECT symbol,
       AVG(price) AS moving_avg,
       MAX(price) AS max_price
FROM StockTrades
WINDOW SLIDING(10 sec, 2 sec)
GROUP BY symbol
INSERT INTO MovingAverages;
```

**Parameters:**
- First: Window size
- Second: Slide interval

**Visual Representation:**
```
Events: ──●──●──●──●──●──●──●──●──▶
Windows: [────────────]
           [────────────]
             [────────────]
               [────────────]
```

### Session Window

Groups events with gaps shorter than the timeout. Sessions end after inactivity.

```sql
-- User sessions with 30-minute timeout
SELECT user_id,
       COUNT(*) AS click_count,
       MIN(timestamp) AS session_start,
       MAX(timestamp) AS session_end
FROM ClickStream
WINDOW SESSION(30 min)
GROUP BY user_id
INSERT INTO UserSessions;
```

**Use Cases:**
- User activity sessions
- Device connectivity windows
- Transaction sequences

**Visual Representation:**
```
Events: ●●●●     ●●     ●●●●●●●       ●●●●
Sessions: [──────] [─]   [─────────]   [────]
          Session1  S2      Session3     S4
```

### Time Window

Continuous rolling window based on event time.

```sql
-- Rolling 1-minute window
SELECT sensor_id,
       AVG(value) AS rolling_avg
FROM Readings
WINDOW TIME(1 min)
GROUP BY sensor_id
INSERT INTO RollingStats;
```

### TimeBatch Window

Batches events and emits at fixed intervals.

```sql
-- Emit batch every 10 seconds
SELECT symbol,
       SUM(volume) AS total_volume,
       COUNT(*) AS trade_count
FROM Trades
WINDOW TIMEBATCH(10 sec)
GROUP BY symbol
INSERT INTO BatchedStats;
```

### ExternalTime Window

Uses a timestamp attribute from the event for windowing (event time vs processing time).

```sql
-- Use event timestamp for windowing
SELECT device_id,
       AVG(temperature) AS avg_temp
FROM SensorData
WINDOW EXTERNALTIME(event_time, 5 min)
GROUP BY device_id
INSERT INTO Stats;
```

**Parameters:**
- First: Timestamp attribute name
- Second: Window duration

---

## Count-Based Windows

### Length Window

Maintains a sliding window of the last N events.

```sql
-- Keep last 100 trades per symbol
SELECT symbol,
       AVG(price) AS avg_price,
       STDDEV(price) AS price_stddev
FROM StockTrades
WINDOW LENGTH(100)
GROUP BY symbol
INSERT INTO RecentStats;
```

**Visual Representation:**
```
Events: 1 2 3 4 5 6 7 8 9 ...
Window:     [3 4 5 6 7]      (length=5)
               [4 5 6 7 8]
                  [5 6 7 8 9]
```

### LengthBatch Window

Collects N events, emits as batch, then resets.

```sql
-- Emit after every 50 events
SELECT symbol,
       AVG(price) AS batch_avg,
       SUM(volume) AS batch_volume
FROM Trades
WINDOW LENGTHBATCH(50)
GROUP BY symbol
INSERT INTO BatchResults;
```

**Visual Representation:**
```
Events:  1 2 3 4 5 | 6 7 8 9 10 | 11 ...
Batches:   Batch 1      Batch 2
          [─────]      [───────]
```

---

## Special Windows

### Delay Window

Delays event emission by a specified duration. Useful for handling late arrivals.

```sql
-- Delay events by 30 seconds
SELECT *
FROM SensorReadings
WINDOW DELAY(30 sec)
INSERT INTO DelayedReadings;
```

---

## Combining Windows with GROUP BY

Windows work naturally with GROUP BY for partitioned aggregations:

```sql
SELECT
    region,
    device_type,
    AVG(latency) AS avg_latency,
    MAX(latency) AS max_latency,
    COUNT(*) AS request_count
FROM NetworkRequests
WINDOW TUMBLING(1 min)
GROUP BY region, device_type
INSERT INTO RegionalStats;
```

## Window with HAVING

Filter aggregated results:

```sql
SELECT symbol,
       AVG(price) AS avg_price,
       COUNT(*) AS trade_count
FROM Trades
WINDOW TUMBLING(5 min)
GROUP BY symbol
HAVING COUNT(*) > 10 AND AVG(price) > 100
INSERT INTO ActiveHighValueStocks;
```

## Best Practices

:::tip Window Selection Guide

| Scenario | Recommended Window |
|----------|-------------------|
| Periodic reports (hourly/daily) | Tumbling |
| Moving averages | Sliding |
| User session analysis | Session |
| Recent event history | Length |
| Batch processing | LengthBatch or TimeBatch |
| Out-of-order events | ExternalTime |
| Late arrival handling | Delay |

:::

:::caution Memory Considerations

- **Large windows** consume more memory
- **Session windows** can grow unbounded for active keys
- **Length windows** have predictable memory usage
- Monitor memory usage in production

:::

## Next Steps

- **[Aggregations](/docs/sql-reference/aggregations)** - Aggregate functions for windows
- **[Joins](/docs/sql-reference/joins)** - Joining windowed streams
- **[Patterns](/docs/sql-reference/patterns)** - Pattern detection across windows
