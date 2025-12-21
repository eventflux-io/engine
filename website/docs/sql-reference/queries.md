---
sidebar_position: 1
title: SQL Query Reference
description: Complete reference for EventFlux SQL queries
---

# SQL Query Reference

EventFlux uses an extended SQL syntax designed for stream processing. This reference covers all supported query constructs.

## Stream Definitions

### DEFINE STREAM

Create a new stream with a schema:

```sql
DEFINE STREAM StreamName (
    attribute1 TYPE,
    attribute2 TYPE,
    ...
);
```

**Supported Types:**

| Type | Description | Example |
|------|-------------|---------|
| `INT` | 32-bit integer | `42` |
| `LONG` | 64-bit integer | `9223372036854775807` |
| `FLOAT` | 32-bit floating point | `3.14` |
| `DOUBLE` | 64-bit floating point | `3.14159265359` |
| `STRING` | UTF-8 text | `'hello'` |
| `BOOL` | Boolean | `true`, `false` |

**Example:**

```sql
DEFINE STREAM StockTrades (
    symbol STRING,
    price DOUBLE,
    volume INT,
    timestamp LONG
);
```

### DEFINE TABLE

Create a table for storing and querying reference data:

```sql
DEFINE TABLE TableName (
    attribute1 TYPE,
    attribute2 TYPE,
    ...
);
```

Tables can be joined with streams for enrichment:

```sql
DEFINE TABLE StockInfo (
    symbol STRING,
    company_name STRING,
    sector STRING
);

SELECT t.symbol, t.price, s.company_name
FROM Trades AS t
JOIN StockInfo AS s
  ON t.symbol = s.symbol
INSERT INTO EnrichedTrades;
```

## Basic Queries

### SELECT Statement

```sql
SELECT attribute1, attribute2, expression AS alias
FROM StreamName
WHERE condition
INSERT INTO OutputStream;
```

**Example:**

```sql
SELECT symbol, price, volume * price AS total_value
FROM StockTrades
WHERE price > 100.0
INSERT INTO HighValueTrades;
```

### SELECT with Aliases

```sql
SELECT
    s.symbol,
    s.price AS trade_price,
    s.volume * s.price AS notional
FROM StockTrades AS s
INSERT INTO ProcessedTrades;
```

## Operators

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

<Tabs>
  <TabItem value="arithmetic" label="Arithmetic" default>

| Operator | Description | Example |
|----------|-------------|---------|
| `+` | Addition | `price + fee` |
| `-` | Subtraction | `high - low` |
| `*` | Multiplication | `price * volume` |
| `/` | Division | `total / count` |
| `%` | Modulo | `id % 10` |

  </TabItem>
  <TabItem value="comparison" label="Comparison">

| Operator | Description | Example |
|----------|-------------|---------|
| `=` | Equal | `symbol = 'AAPL'` |
| `!=`, `<>` | Not equal | `status != 'CLOSED'` |
| `<` | Less than | `price < 100` |
| `>` | Greater than | `volume > 1000` |
| `<=` | Less than or equal | `temp <= 0` |
| `>=` | Greater than or equal | `count >= 5` |

  </TabItem>
  <TabItem value="logical" label="Logical">

| Operator | Description | Example |
|----------|-------------|---------|
| `AND` | Logical AND | `a > 0 AND b > 0` |
| `OR` | Logical OR | `status = 'A' OR status = 'B'` |
| `NOT` | Logical NOT | `NOT is_deleted` |
| `IS NULL` | Null check | `email IS NULL` |
| `IS NOT NULL` | Not null check | `name IS NOT NULL` |

  </TabItem>
  <TabItem value="string" label="String">

| Operator | Description | Example |
|----------|-------------|---------|
| `LIKE` | Pattern match | `name LIKE 'J%'` |
| `IN` | Set membership | `status IN ('A', 'B')` |
| `BETWEEN` | Range check | `price BETWEEN 10 AND 100` |

  </TabItem>
</Tabs>

## Expressions

### Mathematical Functions

```sql
SELECT
    ABS(price_change) AS abs_change,
    SQRT(variance) AS std_dev,
    POWER(growth_rate, 2) AS squared_growth,
    ROUND(price, 2) AS rounded_price
FROM DataStream
INSERT INTO Calculations;
```

### String Functions

```sql
SELECT
    UPPER(symbol) AS upper_symbol,
    LOWER(name) AS lower_name,
    LENGTH(description) AS desc_length,
    CONCAT(first_name, ' ', last_name) AS full_name
FROM DataStream
INSERT INTO Processed;
```

### Conditional Expressions

EventFlux supports both **Searched CASE** and **Simple CASE** syntax:

```sql
-- Searched CASE: boolean conditions
SELECT symbol,
       CASE
           WHEN price > 100 THEN 'HIGH'
           WHEN price > 50 THEN 'MEDIUM'
           ELSE 'LOW'
       END AS price_tier
FROM StockTrades
INSERT INTO Categorized;

-- Simple CASE: value matching
SELECT symbol,
       CASE status
           WHEN 'ACTIVE' THEN 1
           WHEN 'PENDING' THEN 2
           ELSE 0
       END AS status_code
FROM Orders
INSERT INTO StatusCodes;
```

See [Functions Reference](/docs/sql-reference/functions#case-expression) for advanced CASE patterns including nested expressions.

### COALESCE and NULLIF

```sql
SELECT
    COALESCE(nickname, name, 'Unknown') AS display_name,
    NULLIF(status, 'UNKNOWN') AS valid_status
FROM Users
INSERT INTO ProcessedUsers;
```

## INSERT INTO

All queries must specify an output destination:

```sql
-- Insert into a stream
SELECT * FROM Input INSERT INTO OutputStream;

-- The output stream is automatically created if not defined
```

## Multiple Queries

An EventFlux application can contain multiple queries:

```sql
DEFINE STREAM RawTrades (symbol STRING, price DOUBLE, volume INT);

-- Query 1: Filter high-value trades
SELECT symbol, price, volume
FROM RawTrades
WHERE price * volume > 10000
INSERT INTO HighValueTrades;

-- Query 2: Compute statistics
SELECT symbol, AVG(price) AS avg_price
FROM RawTrades
WINDOW TUMBLING(1 min)
GROUP BY symbol
INSERT INTO TradeStats;

-- Query 3: Alert on anomalies
SELECT symbol, avg_price
FROM TradeStats
WHERE avg_price > 500
INSERT INTO PriceAlerts;
```

## Comments

```sql
-- This is a single-line comment

/* This is a
   multi-line comment */

DEFINE STREAM Input (
    value INT  -- inline comment
);
```

## Best Practices

:::tip Query Design Tips

1. **Filter early** - Apply WHERE clauses as close to the source as possible
2. **Use appropriate windows** - Choose window type based on your use case
3. **Avoid SELECT \*** - Explicitly list needed columns for better performance
4. **Name your outputs** - Use meaningful INSERT INTO targets for clarity

:::

## Next Steps

- **[Windows](/docs/sql-reference/windows)** - Time and count-based windowing
- **[Aggregations](/docs/sql-reference/aggregations)** - GROUP BY and aggregate functions
- **[Joins](/docs/sql-reference/joins)** - Stream-to-stream and stream-to-table joins
- **[Patterns](/docs/sql-reference/patterns)** - Complex event pattern detection
- **[Functions](/docs/sql-reference/functions)** - Built-in function reference
