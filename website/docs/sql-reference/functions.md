---
sidebar_position: 6
title: Functions
description: Built-in functions reference for Eventflux
---

# Functions Reference

Eventflux provides a comprehensive set of built-in functions for data transformation, mathematical operations, string manipulation, and more.

## Mathematical Functions

### Basic Math

| Function | Description | Example | Result |
|----------|-------------|---------|--------|
| `ABS(x)` | Absolute value | `ABS(-5)` | `5` |
| `CEIL(x)` | Round up | `CEIL(4.2)` | `5` |
| `FLOOR(x)` | Round down | `FLOOR(4.8)` | `4` |
| `ROUND(x)` | Round to nearest | `ROUND(4.5)` | `5` |
| `ROUND(x, n)` | Round to n decimals | `ROUND(4.567, 2)` | `4.57` |

### Advanced Math

| Function | Description | Example |
|----------|-------------|---------|
| `SQRT(x)` | Square root | `SQRT(16)` → `4` |
| `POWER(x, y)` | x raised to y | `POWER(2, 3)` → `8` |
| `EXP(x)` | e raised to x | `EXP(1)` → `2.718...` |
| `LN(x)` | Natural logarithm | `LN(2.718)` → `1` |
| `LOG(x)` | Base-10 logarithm | `LOG(100)` → `2` |
| `LOG(base, x)` | Logarithm with base | `LOG(2, 8)` → `3` |

### Trigonometric Functions

| Function | Description |
|----------|-------------|
| `SIN(x)` | Sine (radians) |
| `COS(x)` | Cosine (radians) |
| `TAN(x)` | Tangent (radians) |
| `ASIN(x)` | Arc sine |
| `ACOS(x)` | Arc cosine |
| `ATAN(x)` | Arc tangent |

### Example

```sql
SELECT sensor_id,
       ABS(delta) AS abs_delta,
       SQRT(variance) AS std_dev,
       ROUND(value, 2) AS rounded_value,
       POWER(growth_rate, 2) AS squared_growth
FROM Measurements
INSERT INTO Processed;
```

## String Functions

### Basic String Operations

| Function | Description | Example | Result |
|----------|-------------|---------|--------|
| `LENGTH(s)` | String length | `LENGTH('hello')` | `5` |
| `UPPER(s)` | Uppercase | `UPPER('hello')` | `'HELLO'` |
| `LOWER(s)` | Lowercase | `LOWER('HELLO')` | `'hello'` |
| `TRIM(s)` | Remove whitespace | `TRIM('  hi  ')` | `'hi'` |

### String Manipulation

| Function | Description | Example | Result |
|----------|-------------|---------|--------|
| `CONCAT(a, b, ...)` | Concatenate | `CONCAT('a', 'b', 'c')` | `'abc'` |
| `SUBSTRING(s, start, len)` | Extract substring | `SUBSTRING('hello', 1, 3)` | `'hel'` |
| `REPLACE(s, from, to)` | Replace text | `REPLACE('hello', 'l', 'x')` | `'hexxo'` |
| `REVERSE(s)` | Reverse string | `REVERSE('hello')` | `'olleh'` |

### String Searching

| Function | Description | Example | Result |
|----------|-------------|---------|--------|
| `POSITION(sub IN s)` | Find position | `POSITION('ll' IN 'hello')` | `3` |
| `STARTS_WITH(s, prefix)` | Check prefix | `STARTS_WITH('hello', 'he')` | `true` |
| `ENDS_WITH(s, suffix)` | Check suffix | `ENDS_WITH('hello', 'lo')` | `true` |
| `CONTAINS(s, sub)` | Contains check | `CONTAINS('hello', 'ell')` | `true` |

### Example

```sql
SELECT user_id,
       UPPER(first_name) AS first_name,
       LOWER(email) AS email,
       CONCAT(first_name, ' ', last_name) AS full_name,
       LENGTH(description) AS desc_length
FROM Users
INSERT INTO ProcessedUsers;
```

## Conditional Functions

### CASE Expression

```sql
SELECT symbol,
       price,
       CASE
           WHEN price > 1000 THEN 'EXPENSIVE'
           WHEN price > 100 THEN 'MODERATE'
           WHEN price > 10 THEN 'CHEAP'
           ELSE 'PENNY'
       END AS price_category
FROM Stocks
INSERT INTO Categorized;
```

### COALESCE

Returns the first non-null value:

```sql
SELECT user_id,
       COALESCE(nickname, username, email, 'Anonymous') AS display_name
FROM Users
INSERT INTO DisplayNames;
```

### NULLIF

Returns null if values are equal:

```sql
SELECT order_id,
       NULLIF(status, 'UNKNOWN') AS valid_status,
       total / NULLIF(quantity, 0) AS unit_price  -- Avoid division by zero
FROM Orders
INSERT INTO Processed;
```

### IF / IIF

Conditional value selection:

```sql
SELECT symbol,
       price,
       IF(price > previous_price, 'UP', 'DOWN') AS direction,
       IIF(volume > avg_volume, 'HIGH', 'NORMAL') AS volume_status
FROM Trades
INSERT INTO Analysis;
```

## Type Conversion Functions

### CAST

Convert between types:

```sql
SELECT
    CAST(price AS INT) AS price_int,
    CAST(quantity AS DOUBLE) AS quantity_double,
    CAST(timestamp AS STRING) AS timestamp_str
FROM Orders
INSERT INTO Converted;
```

### Supported Conversions

| From | To | Example |
|------|-----|---------|
| INT | DOUBLE | `CAST(42 AS DOUBLE)` → `42.0` |
| DOUBLE | INT | `CAST(42.9 AS INT)` → `42` |
| INT | STRING | `CAST(42 AS STRING)` → `'42'` |
| STRING | INT | `CAST('42' AS INT)` → `42` |
| BOOL | INT | `CAST(true AS INT)` → `1` |

## Aggregate Functions

See [Aggregations](/docs/sql-reference/aggregations) for detailed coverage.

| Function | Description |
|----------|-------------|
| `COUNT(*)` | Count all events |
| `COUNT(attr)` | Count non-null |
| `COUNT(DISTINCT attr)` | Count unique |
| `SUM(attr)` | Sum values |
| `AVG(attr)` | Average |
| `MIN(attr)` | Minimum |
| `MAX(attr)` | Maximum |
| `STDDEV(attr)` | Standard deviation |

## Date/Time Functions

### Current Time

| Function | Description |
|----------|-------------|
| `CURRENT_TIMESTAMP` | Current timestamp |
| `NOW()` | Current timestamp |

### Time Extraction

```sql
SELECT event_id,
       EXTRACT(YEAR FROM timestamp) AS year,
       EXTRACT(MONTH FROM timestamp) AS month,
       EXTRACT(DAY FROM timestamp) AS day,
       EXTRACT(HOUR FROM timestamp) AS hour,
       EXTRACT(MINUTE FROM timestamp) AS minute
FROM Events
INSERT INTO TimeParts;
```

### Time Arithmetic

```sql
SELECT event_id,
       timestamp,
       timestamp + INTERVAL '1' HOUR AS plus_one_hour,
       timestamp - INTERVAL '30' MINUTE AS minus_thirty_min
FROM Events
INSERT INTO AdjustedTimes;
```

## Utility Functions

### NULL Handling

```sql
SELECT
    IFNULL(value, 0) AS value_or_zero,
    COALESCE(a, b, c, 'default') AS first_non_null,
    NULLIF(status, 'UNKNOWN') AS null_if_unknown
FROM Data
INSERT INTO Processed;
```

### Type Checking

```sql
SELECT
    value,
    IS_NULL(value) AS is_null,
    IS_NOT_NULL(value) AS is_not_null
FROM Data
INSERT INTO Checks;
```

## Examples

### Financial Calculations

```sql
SELECT symbol,
       price,
       volume,
       price * volume AS notional,
       ROUND(price * volume / 1000000, 2) AS notional_millions,
       ABS(price - previous_close) / previous_close * 100 AS pct_change
FROM MarketData
INSERT INTO Calculations;
```

### Text Processing

```sql
SELECT order_id,
       UPPER(TRIM(customer_name)) AS customer_name,
       CONCAT(
           SUBSTRING(phone, 1, 3), '-',
           SUBSTRING(phone, 4, 3), '-',
           SUBSTRING(phone, 7, 4)
       ) AS formatted_phone
FROM Orders
INSERT INTO FormattedOrders;
```

### Data Cleansing

```sql
SELECT user_id,
       COALESCE(NULLIF(TRIM(email), ''), 'unknown@example.com') AS email,
       CASE
           WHEN age < 0 THEN NULL
           WHEN age > 150 THEN NULL
           ELSE age
       END AS valid_age,
       UPPER(COALESCE(country, 'UNKNOWN')) AS country
FROM RawUsers
INSERT INTO CleanedUsers;
```

## Best Practices

:::tip Function Usage

1. **Use COALESCE for defaults** - Handle nulls gracefully
2. **Validate before CAST** - Avoid runtime errors
3. **Filter early** - Apply functions after WHERE when possible
4. **Use appropriate precision** - ROUND for display, keep full precision for calculations

:::

## Next Steps

- **[SQL Reference](/docs/sql-reference/queries)** - Complete query syntax
- **[Aggregations](/docs/sql-reference/aggregations)** - Aggregate functions
- **[Patterns](/docs/sql-reference/patterns)** - Use functions in pattern filters
