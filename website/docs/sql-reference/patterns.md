---
sidebar_position: 5
title: Pattern Matching
description: Complex event pattern detection in Eventflux
---

# Pattern Matching

Pattern matching enables detection of complex event sequences across streams. Eventflux provides powerful pattern matching capabilities with temporal constraints, filters, and flexible output selection.

:::info Production Ready
Pattern matching is tested with **370+ tests** covering sequences, timing constraints, filters, and edge cases.
:::

## Pattern Syntax

```sql
FROM StreamName
MATCH (pattern_definition)
WITHIN time_constraint
FILTER conditions
SELECT output_attributes
INSERT INTO OutputStream;
```

## Basic Patterns

### Sequential Events

Detect events in sequence using the `->` operator:

```sql
-- Detect two consecutive events
FROM SensorReadings
MATCH (e1=Reading -> e2=Reading)
WITHIN 1 min
SELECT e1.sensor_id, e1.value AS first_value, e2.value AS second_value
INSERT INTO Sequences;
```

### Named Event Variables

Assign names to pattern elements for reference:

```sql
FROM StockTrades
MATCH (buy=Trade -> sell=Trade)
WITHIN 5 min
FILTER buy.side = 'BUY' AND sell.side = 'SELL'
   AND buy.symbol = sell.symbol
SELECT buy.symbol,
       buy.price AS buy_price,
       sell.price AS sell_price,
       sell.price - buy.price AS profit
INSERT INTO TradePairs;
```

## Pattern Operators

### Sequence (`->`)

Events must occur in order:

```sql
MATCH (a=Event -> b=Event -> c=Event)
-- a happens before b, b happens before c
```

### Any Order

Events can occur in any order within the time window:

```sql
MATCH (a=Event, b=Event, c=Event)
-- a, b, c all occur, but in any order
```

## Time Constraints

### WITHIN Clause

Specifies the maximum time span for the entire pattern:

```sql
FROM Events
MATCH (start=Event -> middle=Event -> end=Event)
WITHIN 10 min  -- All three events must occur within 10 minutes
SELECT start.id, end.id
INSERT INTO Matches;
```

**Time Units:**
- `sec` / `second` / `seconds`
- `min` / `minute` / `minutes`
- `hour` / `hours`
- `day` / `days`

## Filter Conditions

### FILTER Clause

Apply conditions to pattern elements:

```sql
FROM SensorReadings
MATCH (e1=Reading -> e2=Reading -> e3=Reading)
WITHIN 5 min
FILTER e1.sensor_id = e2.sensor_id
   AND e2.sensor_id = e3.sensor_id
   AND e2.temperature > e1.temperature
   AND e3.temperature > e2.temperature
SELECT e1.sensor_id,
       e1.temperature AS temp1,
       e2.temperature AS temp2,
       e3.temperature AS temp3
INSERT INTO TemperatureSpikes;
```

### Complex Conditions

```sql
FROM Transactions
MATCH (t1=Transaction -> t2=Transaction -> t3=Transaction)
WITHIN 1 hour
FILTER t1.account_id = t2.account_id
   AND t2.account_id = t3.account_id
   AND t1.amount > 1000
   AND t2.amount > 1000
   AND t3.amount > 1000
   AND t1.location != t2.location
   AND t2.location != t3.location
SELECT t1.account_id,
       t1.amount + t2.amount + t3.amount AS total_amount,
       t1.location AS loc1,
       t2.location AS loc2,
       t3.location AS loc3
INSERT INTO SuspiciousActivity;
```

## Pattern Examples

### Fraud Detection

```sql
-- Detect rapid transaction burst
FROM CardTransactions
MATCH (t1=Transaction -> t2=Transaction -> t3=Transaction -> t4=Transaction -> t5=Transaction)
WITHIN 10 min
FILTER t1.card_id = t2.card_id
   AND t2.card_id = t3.card_id
   AND t3.card_id = t4.card_id
   AND t4.card_id = t5.card_id
SELECT t1.card_id,
       COUNT(*) AS transaction_count,
       t1.amount + t2.amount + t3.amount + t4.amount + t5.amount AS total_amount
INSERT INTO FraudAlerts;
```

### IoT Anomaly Detection

```sql
-- Detect temperature spike followed by pressure drop
FROM IndustrialSensors
MATCH (temp_high=TempReading -> pressure_low=PressureReading)
WITHIN 30 sec
FILTER temp_high.device_id = pressure_low.device_id
   AND temp_high.value > 100
   AND pressure_low.value < 20
SELECT temp_high.device_id,
       temp_high.value AS temperature,
       pressure_low.value AS pressure,
       'CRITICAL' AS alert_level
INSERT INTO EquipmentAlerts;
```

### Price Momentum

```sql
-- Detect three consecutive price increases
FROM StockTicks
MATCH (e1=Tick -> e2=Tick -> e3=Tick)
WITHIN 1 min
FILTER e1.symbol = e2.symbol
   AND e2.symbol = e3.symbol
   AND e2.price > e1.price
   AND e3.price > e2.price
SELECT e1.symbol,
       e1.price AS price1,
       e2.price AS price2,
       e3.price AS price3,
       (e3.price - e1.price) / e1.price * 100 AS pct_gain
INSERT INTO MomentumSignals;
```

### User Journey Analysis

```sql
-- Detect signup -> browse -> purchase journey
FROM UserEvents
MATCH (signup=Event -> browse=Event -> purchase=Event)
WITHIN 24 hours
FILTER signup.user_id = browse.user_id
   AND browse.user_id = purchase.user_id
   AND signup.event_type = 'SIGNUP'
   AND browse.event_type = 'BROWSE'
   AND purchase.event_type = 'PURCHASE'
SELECT signup.user_id,
       signup.timestamp AS signup_time,
       purchase.timestamp AS purchase_time,
       purchase.amount AS first_purchase_amount
INSERT INTO ConversionJourneys;
```

### System Health Monitoring

```sql
-- Detect warning followed by error followed by critical
FROM SystemLogs
MATCH (warn=Log -> error=Log -> critical=Log)
WITHIN 5 min
FILTER warn.service = error.service
   AND error.service = critical.service
   AND warn.level = 'WARN'
   AND error.level = 'ERROR'
   AND critical.level = 'CRITICAL'
SELECT critical.service,
       warn.message AS warning_msg,
       error.message AS error_msg,
       critical.message AS critical_msg,
       critical.timestamp
INSERT INTO EscalationAlerts;
```

### Order Flow Pattern

```sql
-- Detect order -> partial fill -> complete fill
FROM OrderEvents
MATCH (order=Event -> partial=Event -> complete=Event)
WITHIN 1 hour
FILTER order.order_id = partial.order_id
   AND partial.order_id = complete.order_id
   AND order.event_type = 'NEW_ORDER'
   AND partial.event_type = 'PARTIAL_FILL'
   AND complete.event_type = 'FILLED'
SELECT order.order_id,
       order.symbol,
       order.quantity AS ordered_qty,
       complete.filled_qty,
       complete.avg_price
INSERT INTO OrderLifecycle;
```

## Pattern Behavior

### Event Consumption

By default, each event can participate in multiple pattern matches. This allows overlapping pattern detection.

### Pattern State

Patterns maintain state for active (incomplete) matches within the time window. State is automatically cleaned up when:
- The time window expires
- A pattern is completed
- No matching continuation is possible

## Best Practices

:::tip Pattern Design

1. **Be specific with filters** - Narrow down matches early
2. **Use appropriate time windows** - Balance detection latency vs resource usage
3. **Name events meaningfully** - Makes SELECT and FILTER clearer
4. **Test edge cases** - Verify behavior with out-of-order events

:::

:::caution Performance Considerations

- **Pattern complexity** - More elements = more state
- **Time window size** - Larger windows retain more partial matches
- **Filter selectivity** - Filters applied early reduce state
- **Event volume** - High-throughput streams need efficient patterns

:::

## Next Steps

- **[Windows](/docs/sql-reference/windows)** - Combine patterns with windows
- **[Aggregations](/docs/sql-reference/aggregations)** - Aggregate pattern outputs
- **[Functions](/docs/sql-reference/functions)** - Use functions in filters
