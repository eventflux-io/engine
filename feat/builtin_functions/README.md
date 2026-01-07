# Built-in Functions Feature Document

This document provides a comprehensive overview of EventFlux's built-in function system, including what's already
implemented, the architecture, and what needs to be added.

## Current Status (Updated 2025-01-07)

**EventFlux has 70+ built-in functions** - significantly more than Siddhi's core 19 functions.

### Summary by Category

| Category            | Count   | Status                 |
|---------------------|---------|------------------------|
| Math Functions      | 23      | Complete (incl. asin, acos, atan, mod, sign, trunc, maximum, minimum) |
| String Functions    | 22      | Complete (incl. left, right, ltrim, rtrim, reverse, repeat, position, ascii, chr, lpad, rpad) |
| Date/Time Functions | 5       | Complete (incl. now()) |
| Type Conversion     | 5       | Complete (incl. default, ifnull with type widening) |
| Type Checking       | 6       | Complete               |
| Utility Functions   | 4       | Complete               |
| Aggregate Functions | 6       | Complete               |
| **Total**           | **71+** | **100% core complete** |

---

## Architecture Overview

### File Structure

```
src/core/executor/function/
├── mod.rs                           # Module exports
├── builtin_wrapper.rs               # Factory builders and registration
├── scalar_function_executor.rs      # ScalarFunctionExecutor trait
├── math_functions.rs                # Math functions (abs, sqrt, sin, etc.)
├── string_functions.rs              # String functions (concat, upper, etc.)
├── date_functions.rs                # Date/time functions
├── default_function_executor.rs     # DEFAULT/IFNULL implementation
├── cast_function_executor.rs        # Type casting
├── convert_function_executor.rs     # Safe type conversion
├── coalesce_function_executor.rs    # COALESCE implementation
├── nullif_function_executor.rs      # NULLIF implementation
├── uuid_function_executor.rs        # UUID generation
├── event_timestamp_function_executor.rs  # Event timestamp
├── instance_of_checkers.rs          # Type checking functions
└── script_function_executor.rs      # User-defined script functions
```

### Registration Flow

```
EventFluxContext::new()
    └── register_default_extensions()
        └── register_builtin_scalar_functions(ctx)
            └── ctx.add_scalar_function_factory("function_name", factory)
```

**Registration Location:** `src/core/executor/function/builtin_wrapper.rs`

```rust
pub fn register_builtin_scalar_functions(ctx: &EventFluxContext) {
    // Math functions
    ctx.add_scalar_function_factory("abs", build_abs);
    ctx.add_scalar_function_factory("sqrt", build_sqrt);
    // ... more functions
}
```

### Core Traits

**ExpressionExecutor** (base trait):

```rust
pub trait ExpressionExecutor: Debug + Send + Sync + 'static {
    fn execute(&self, event: Option<&dyn ComplexEvent>) -> Option<AttributeValue>;
    fn get_return_type(&self) -> ApiAttributeType;
    fn clone_executor(&self, ctx: &Arc<EventFluxAppContext>) -> Box<dyn ExpressionExecutor>;
}
```

**ScalarFunctionExecutor** (for built-in functions):

```rust
pub trait ScalarFunctionExecutor: ExpressionExecutor {
    fn init(&mut self, args: &Vec<Box<dyn ExpressionExecutor>>, ctx: &Arc<EventFluxAppContext>) -> Result<(), String>;
    fn destroy(&mut self) {}
    fn get_name(&self) -> String;
    fn clone_scalar_function(&self) -> Box<dyn ScalarFunctionExecutor>;
}
```

---

## Functions Reference

### Math Functions (23)

#### Basic Math

| Function | Signature                               | Return Type   | Description               |
|----------|-----------------------------------------|---------------|---------------------------|
| `abs`    | `abs(num)`                              | Same as input | Absolute value            |
| `ceil`   | `ceil(num)`                             | DOUBLE        | Round up to nearest int   |
| `floor`  | `floor(num)`                            | DOUBLE        | Round down to nearest int |
| `round`  | `round(num)` or `round(num, precision)` | DOUBLE        | Round to nearest          |
| `trunc`  | `trunc(num)` or `trunc(num, precision)` | DOUBLE        | Truncate decimal places   |
| `sign`   | `sign(num)`                             | INT           | Returns -1, 0, or 1       |
| `mod`    | `mod(a, b)`                             | DOUBLE        | Modulo (remainder)        |

**Aliases:** `truncate` → `trunc`

#### Comparison

| Function  | Signature            | Return Type | Description                |
|-----------|----------------------|-------------|----------------------------|
| `maximum` | `maximum(a, b, ...)` | DOUBLE      | Maximum of multiple values |
| `minimum` | `minimum(a, b, ...)` | DOUBLE      | Minimum of multiple values |

#### Exponential & Logarithmic

| Function | Signature          | Return Type | Description         |
|----------|--------------------|-------------|---------------------|
| `sqrt`   | `sqrt(num)`        | DOUBLE      | Square root         |
| `power`  | `power(base, exp)` | DOUBLE      | x raised to power y |
| `exp`    | `exp(num)`         | DOUBLE      | e raised to power x |
| `ln`     | `ln(num)`          | DOUBLE      | Natural logarithm   |
| `log`    | `log(num)`         | DOUBLE      | Natural logarithm   |
| `log10`  | `log10(num)`       | DOUBLE      | Base-10 logarithm   |

**Aliases:** `pow` → `power`, `ln` → `log`

#### Trigonometric

| Function | Signature  | Return Type | Description                           |
|----------|------------|-------------|---------------------------------------|
| `sin`    | `sin(num)` | DOUBLE      | Sine (radians)                        |
| `cos`    | `cos(num)` | DOUBLE      | Cosine (radians)                      |
| `tan`    | `tan(num)` | DOUBLE      | Tangent (radians)                     |
| `asin`   | `asin(num)`| DOUBLE      | Arc sine (returns NaN if out of [-1,1]) |
| `acos`   | `acos(num)`| DOUBLE      | Arc cosine (returns NaN if out of [-1,1]) |
| `atan`   | `atan(num)`| DOUBLE      | Arc tangent                           |

---

### String Functions (19)

#### Basic Operations

| Function | Signature     | Return Type | Description          |
|----------|---------------|-------------|----------------------|
| `length` | `length(str)` | INT         | String length        |
| `upper`  | `upper(str)`  | STRING      | Convert to uppercase |
| `lower`  | `lower(str)`  | STRING      | Convert to lowercase |

#### Trimming

| Function | Signature    | Return Type | Description                |
|----------|--------------|-------------|----------------------------|
| `trim`   | `trim(str)`  | STRING      | Remove whitespace (both)   |
| `ltrim`  | `ltrim(str)` | STRING      | Remove leading whitespace  |
| `rtrim`  | `rtrim(str)` | STRING      | Remove trailing whitespace |

#### Extraction & Manipulation

| Function    | Signature                                      | Return Type | Description                         |
|-------------|------------------------------------------------|-------------|-------------------------------------|
| `substring` | `substring(str, start)` or `(str, start, len)` | STRING      | Extract substring                   |
| `left`      | `left(str, n)`                                 | STRING      | Get leftmost n chars                |
| `right`     | `right(str, n)`                                | STRING      | Get rightmost n chars               |
| `lpad`      | `lpad(str, len, pad)`                          | STRING      | Left-pad string to length           |
| `rpad`      | `rpad(str, len, pad)`                          | STRING      | Right-pad string to length          |
| `reverse`   | `reverse(str)`                                 | STRING      | Reverse string                      |
| `repeat`    | `repeat(str, n)`                               | STRING      | Repeat string n times               |

**Aliases:** `substr` → `substring`

#### Concatenation & Replacement

| Function  | Signature                | Return Type | Description         |
|-----------|--------------------------|-------------|---------------------|
| `concat`  | `concat(str, str, ...)`  | STRING      | Concatenate strings |
| `replace` | `replace(str, from, to)` | STRING      | Replace text        |

#### Searching

| Function   | Signature               | Return Type | Description             |
|------------|-------------------------|-------------|-------------------------|
| `position` | `position(substr, str)` | INT         | Find position (1-based) |
| `like`     | `like(str, pattern)`    | BOOL        | Pattern matching        |

**Aliases:** `locate` → `position`, `instr` → `position`

#### Character Functions

| Function | Signature    | Return Type | Description               |
|----------|--------------|-------------|---------------------------|
| `ascii`  | `ascii(str)` | INT         | ASCII code of first char  |
| `chr`    | `chr(code)`  | STRING      | Character from ASCII code |

**Aliases:** `char` → `chr`

---

### Date/Time Functions (5)

| Function         | Signature                             | Return Type | Description            |
|------------------|---------------------------------------|-------------|------------------------|
| `now`            | `now()`                               | LONG        | Current time in millis |
| `eventTimestamp` | `eventTimestamp()` or `(event)`       | LONG        | Event's timestamp      |
| `formatDate`     | `formatDate(timestamp, pattern)`      | STRING      | Format timestamp       |
| `parseDate`      | `parseDate(datestr, pattern)`         | LONG        | Parse date string      |
| `dateAdd`        | `dateAdd(timestamp, increment, unit)` | LONG        | Add time to timestamp  |

---

### Type Conversion Functions (4)

| Function  | Signature                    | Return Type         | Description                   |
|-----------|------------------------------|---------------------|-------------------------------|
| `cast`    | `cast(value, 'typename')`    | Target type         | Convert type (may fail)       |
| `convert` | `convert(value, 'typename')` | Target type         | Safe type conversion          |
| `nullif`  | `nullif(a, b)`               | Type of `a` or NULL | NULL if values are equal      |
| `default` | `default(val, defaultVal)`   | Widest numeric type | Return default if val is null |

**Aliases:** `ifnull` → `default`

**Supported Type Names:** `'string'`, `'int'`, `'long'`, `'float'`, `'double'`, `'bool'`, `'boolean'`, `'object'`

**Type Widening:** `default()` and `ifnull()` support automatic numeric type widening (INT → LONG → FLOAT → DOUBLE).
For example, `default(int_column, 999999999999)` works even though the default is a LONG.

---

### Type Checking Functions (6)

| Function            | Signature                | Return Type |
|---------------------|--------------------------|-------------|
| `instanceOfBoolean` | `instanceOfBoolean(val)` | BOOL        |
| `instanceOfString`  | `instanceOfString(val)`  | BOOL        |
| `instanceOfInteger` | `instanceOfInteger(val)` | BOOL        |
| `instanceOfLong`    | `instanceOfLong(val)`    | BOOL        |
| `instanceOfFloat`   | `instanceOfFloat(val)`   | BOOL        |
| `instanceOfDouble`  | `instanceOfDouble(val)`  | BOOL        |

---

### Utility Functions (4)

| Function         | Signature                | Return Type            | Description             |
|------------------|--------------------------|------------------------|-------------------------|
| `coalesce`       | `coalesce(a, b, c, ...)` | Type of first non-null | First non-null value    |
| `uuid`           | `uuid()`                 | STRING                 | Generate UUID           |
| `eventTimestamp` | `eventTimestamp()`       | LONG                   | Current event timestamp |

---

### Aggregate Functions (6)

| Function        | Signature               | Return Type   | Description         |
|-----------------|-------------------------|---------------|---------------------|
| `count`         | `count()` or `count(*)` | LONG          | Count events        |
| `sum`           | `sum(num)`              | Same as input | Sum of values       |
| `avg`           | `avg(num)`              | DOUBLE        | Average             |
| `min`           | `min(num)`              | Same as input | Minimum value       |
| `max`           | `max(num)`              | Same as input | Maximum value       |
| `distinctCount` | `distinctCount(val)`    | LONG          | Count unique values |

---

## SQL CAST Handling

CAST is handled specially in the SQL compiler, not as a regular function:

**SQL Syntax:**

```sql
CAST(expression AS type)
```

**Conversion Location:** `src/sql_compiler/converter.rs`

```rust
SqlExpr::Cast { expr, data_type, ..} => {
let inner_expr = Self::convert_expression(expr, catalog) ?;
let target_type = sql_type_to_attribute_type(data_type)?;
Ok(Expression::cast(inner_expr, target_type))
}
```

**Execution Location:** `src/core/executor/cast_executor.rs`

Supported SQL types: `INT`, `INTEGER`, `BIGINT`, `FLOAT`, `DOUBLE`, `VARCHAR`, `STRING`, `BOOLEAN`, `BOOL`

---

## Remaining Functions (To Implement)

### Advanced Aggregators (Priority 1)

| Function     | Signature         | Description                  | Effort |
|--------------|-------------------|------------------------------|--------|
| `minForever` | `minForever(num)` | All-time minimum (no expiry) | MEDIUM |
| `maxForever` | `maxForever(num)` | All-time maximum (no expiry) | MEDIUM |
| `first`      | `first(val)`      | First value in window        | EASY   |
| `last`       | `last(val)`       | Last value in window         | EASY   |
| `stdDev`     | `stdDev(num)`     | Standard deviation           | MEDIUM |

### Date/Time Functions (Priority 2)

| Function   | Signature                          | Description                | Effort |
|------------|------------------------------------|----------------------------|--------|
| `dateDiff` | `dateDiff(unit, start, end)`       | Difference between dates   | MEDIUM |
| `dateSub`  | `dateSub(timestamp, amount, unit)` | Subtract from timestamp    | EASY   |
| `extract`  | `extract(field FROM timestamp)`    | Extract year/month/day/etc | MEDIUM |

---

## Implementation Guide

### Adding a New Scalar Function

1. **Create executor** in `src/core/executor/function/`:

```rust
// my_function_executor.rs
use super::scalar_function_executor::ScalarFunctionExecutor;
use crate::core::executor::ExpressionExecutor;

#[derive(Debug, Clone)]
pub struct MyFunctionExecutor {
    arg: Box<dyn ExpressionExecutor>,
}

impl MyFunctionExecutor {
    pub fn new(args: Vec<Box<dyn ExpressionExecutor>>) -> Result<Self, String> {
        if args.len() != 1 {
            return Err("myFunction requires exactly 1 argument".to_string());
        }
        Ok(Self { arg: args.into_iter().next().unwrap() })
    }
}

impl ExpressionExecutor for MyFunctionExecutor {
    fn execute(&self, event: Option<&dyn ComplexEvent>) -> Option<AttributeValue> {
        let val = self.arg.execute(event)?;
        // Transform val...
        Some(result)
    }

    fn get_return_type(&self) -> ApiAttributeType {
        ApiAttributeType::STRING // or appropriate type
    }

    fn clone_executor(&self, ctx: &Arc<EventFluxAppContext>) -> Box<dyn ExpressionExecutor> {
        Box::new(self.clone())
    }
}

impl ScalarFunctionExecutor for MyFunctionExecutor {
    fn get_name(&self) -> String { "myFunction".to_string() }
    fn clone_scalar_function(&self) -> Box<dyn ScalarFunctionExecutor> {
        Box::new(self.clone())
    }
    fn init(&mut self, _args: &Vec<Box<dyn ExpressionExecutor>>, _ctx: &Arc<EventFluxAppContext>) -> Result<(), String> {
        Ok(())
    }
}
```

2. **Register in `builtin_wrapper.rs`**:

```rust
pub fn register_builtin_scalar_functions(ctx: &EventFluxContext) {
    // ... existing registrations ...
    ctx.add_scalar_function_factory("myFunction", build_my_function);
}
```

3. **Add SQL mapping in `converter.rs`** (if different name):

```rust
"my_function" | "myfunction" => {
Ok(Expression::function_no_ns("myFunction".to_string(), args))
}
```

4. **Add type inference** in `type_inference.rs` (if needed):

```rust
"myFunction" => Ok(AttributeType::STRING),
```

5. **Add tests** in `tests/compatibility/functions/` (organized by category: math_functions.rs, string_functions.rs, utility_functions.rs, etc.).

---

## References

- **EventFlux Functions**: `src/core/executor/function/`
- **SQL Converter**: `src/sql_compiler/converter.rs`
- **Type Inference**: `src/sql_compiler/type_inference.rs`
- **Function Tests**: `tests/compatibility/functions/` (organized by category)
