# Struct Type Implementation

This document specifies the implementation of struct types and the struct() function in EventFlux.

## Purpose

Struct types allow returning composite values from expressions. Required for:
- Returning multiple values from CASE branches
- ai_decide() return type
- Complex event transformations

## Syntax

### struct() Function

```sql
struct('BLOCK', 1.0, 'Reason text')
-- Returns: {field_0: 'BLOCK', field_1: 1.0, field_2: 'Reason text'}

struct(action, confidence, reasoning)
-- Returns: {field_0: action_value, field_1: confidence_value, field_2: reasoning_value}
```

### Named Fields (Optional Enhancement)

```sql
struct(action => 'BLOCK', confidence => 1.0, reasoning => 'Reason text')
-- Returns: {action: 'BLOCK', confidence: 1.0, reasoning: 'Reason text'}
```

### Field Access

```sql
SELECT decision.action, decision.confidence
FROM (
    SELECT struct('BLOCK', 1.0, 'Reason') as decision
    FROM Stream
);
```

## Components to Implement

### 1. AttributeValue::Struct Variant

Location: src/core/event/value.rs

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum AttributeValue {
    Int(i64),
    Long(i64),
    Float(f32),
    Double(f64),
    Bool(bool),
    String(String),
    Object(Vec<u8>),
    Null,
    // Add:
    Struct(StructValue),
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructValue {
    /// Field names (empty for positional access)
    pub field_names: Vec<String>,
    /// Field values
    pub values: Vec<AttributeValue>,
}

impl StructValue {
    pub fn new(values: Vec<AttributeValue>) -> Self {
        let field_names: Vec<String> = (0..values.len())
            .map(|i| format!("field_{}", i))
            .collect();
        Self { field_names, values }
    }

    pub fn with_names(field_names: Vec<String>, values: Vec<AttributeValue>) -> Self {
        assert_eq!(field_names.len(), values.len());
        Self { field_names, values }
    }

    pub fn get_field(&self, name: &str) -> Option<&AttributeValue> {
        self.field_names
            .iter()
            .position(|n| n == name)
            .map(|idx| &self.values[idx])
    }

    pub fn get_field_by_index(&self, index: usize) -> Option<&AttributeValue> {
        self.values.get(index)
    }
}
```

### 2. Type System Addition

Location: src/query_api/definition/attribute.rs

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    INT,
    LONG,
    FLOAT,
    DOUBLE,
    BOOL,
    STRING,
    OBJECT,
    // Add:
    STRUCT(StructType),
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructType {
    pub fields: Vec<(String, Type)>,
}
```

### 3. StructFunctionExecutor

Location: src/core/executor/function/struct_function_executor.rs

```rust
use crate::core::executor::ExpressionExecutor;
use crate::core::event::stream_event::StreamEvent;
use crate::core::event::value::{AttributeValue, StructValue};

pub struct StructFunctionExecutor {
    /// Executors for each field value
    field_executors: Vec<Box<dyn ExpressionExecutor>>,
    /// Optional field names (for named struct)
    field_names: Option<Vec<String>>,
}

impl StructFunctionExecutor {
    pub fn new(field_executors: Vec<Box<dyn ExpressionExecutor>>) -> Self {
        Self {
            field_executors,
            field_names: None,
        }
    }

    pub fn with_names(
        field_names: Vec<String>,
        field_executors: Vec<Box<dyn ExpressionExecutor>>,
    ) -> Self {
        Self {
            field_executors,
            field_names: Some(field_names),
        }
    }
}

impl ExpressionExecutor for StructFunctionExecutor {
    fn execute(&self, event: Option<&StreamEvent>) -> Option<AttributeValue> {
        let values: Vec<AttributeValue> = self
            .field_executors
            .iter()
            .map(|exec| exec.execute(event).unwrap_or(AttributeValue::Null))
            .collect();

        let struct_value = match &self.field_names {
            Some(names) => StructValue::with_names(names.clone(), values),
            None => StructValue::new(values),
        };

        Some(AttributeValue::Struct(struct_value))
    }

    fn get_return_type(&self) -> crate::query_api::definition::attribute::Type {
        // Return OBJECT for now, or implement proper STRUCT type
        crate::query_api::definition::attribute::Type::OBJECT
    }
}
```

### 4. FieldAccessExpressionExecutor

Location: src/core/executor/field_access_expression_executor.rs

```rust
use crate::core::executor::ExpressionExecutor;
use crate::core::event::stream_event::StreamEvent;
use crate::core::event::value::AttributeValue;

pub struct FieldAccessExpressionExecutor {
    /// Executor that returns a Struct
    struct_executor: Box<dyn ExpressionExecutor>,
    /// Field name to access
    field_name: String,
}

impl FieldAccessExpressionExecutor {
    pub fn new(struct_executor: Box<dyn ExpressionExecutor>, field_name: String) -> Self {
        Self {
            struct_executor,
            field_name,
        }
    }
}

impl ExpressionExecutor for FieldAccessExpressionExecutor {
    fn execute(&self, event: Option<&StreamEvent>) -> Option<AttributeValue> {
        let struct_value = self.struct_executor.execute(event)?;

        match struct_value {
            AttributeValue::Struct(s) => s.get_field(&self.field_name).cloned(),
            _ => None, // Not a struct, return NULL
        }
    }

    fn get_return_type(&self) -> crate::query_api::definition::attribute::Type {
        // Cannot determine without struct type info
        crate::query_api::definition::attribute::Type::OBJECT
    }
}
```

### 5. AST Node for Field Access

Location: src/query_api/expression/

```rust
pub enum Expression {
    // existing...
    FieldAccess(FieldAccessExpression),
}

pub struct FieldAccessExpression {
    /// Expression that returns a struct
    pub object: Box<Expression>,
    /// Field name
    pub field: String,
}
```

### 6. Parser Support

#### struct() Function

Register in function registry as a built-in function. The parser already handles function calls.

#### Field Access (dot notation)

Location: sql_compiler or LALRPOP grammar

Need to parse `expr.field` as field access:

```
FieldAccess: Expression = {
    <object:PrimaryExpr> "." <field:Identifier> => {
        Expression::FieldAccess(FieldAccessExpression {
            object: Box::new(object),
            field,
        })
    },
};
```

This conflicts with qualified names (table.column). Resolution:
- If left side is an identifier and matches a known table/alias, treat as qualified name
- Otherwise, treat as field access

Alternative: Use different syntax like `expr->field` or `get_field(expr, 'field')`.

### 7. Function Registry

Location: src/core/executor/function/

Register struct() as built-in:

```rust
// In function registry initialization
registry.register(
    "struct",
    Box::new(StructFunctionFactory),
);

pub struct StructFunctionFactory;

impl FunctionFactory for StructFunctionFactory {
    fn create(
        &self,
        args: Vec<Box<dyn ExpressionExecutor>>,
    ) -> Result<Box<dyn ExpressionExecutor>, String> {
        Ok(Box::new(StructFunctionExecutor::new(args)))
    }
}
```

### 8. Expression Parser Integration

Location: src/core/util/parser/expression_parser.rs

```rust
fn parse_expression(&self, expr: &Expression) -> Result<Box<dyn ExpressionExecutor>, String> {
    match expr {
        // existing...
        Expression::FieldAccess(fa) => self.parse_field_access(fa),
    }
}

fn parse_field_access(
    &self,
    fa: &FieldAccessExpression,
) -> Result<Box<dyn ExpressionExecutor>, String> {
    let struct_exec = self.parse_expression(&fa.object)?;
    Ok(Box::new(FieldAccessExpressionExecutor::new(
        struct_exec,
        fa.field.clone(),
    )))
}
```

### 9. Serialization

StructValue needs serialization support for:
- State persistence (checkpointing)
- Network transport (distributed mode)
- Sink output (JSON, etc.)

```rust
impl StructValue {
    pub fn to_json(&self) -> serde_json::Value {
        let mut map = serde_json::Map::new();
        for (name, value) in self.field_names.iter().zip(self.values.iter()) {
            map.insert(name.clone(), value.to_json());
        }
        serde_json::Value::Object(map)
    }
}
```

## Test Cases

### Basic struct() Creation

```sql
SELECT struct(1, 'hello', 3.14) as data FROM Stream;
-- Result: {field_0: 1, field_1: 'hello', field_2: 3.14}
```

### struct() with Expressions

```sql
SELECT struct(
    transaction_id,
    amount * 1.1,
    concat('User: ', user_id)
) as info
FROM Transactions;
```

### Field Access

```sql
SELECT
    data.field_0 as id,
    data.field_1 as name
FROM (
    SELECT struct(id, name) as data FROM Items
);
```

### struct() in CASE

```sql
SELECT
    CASE
        WHEN score > 90 THEN struct('A', score, 'Excellent')
        WHEN score > 80 THEN struct('B', score, 'Good')
        ELSE struct('C', score, 'Average')
    END as result
FROM Scores;
```

### Nested Field Access

```sql
SELECT result.field_0 as grade, result.field_2 as comment
FROM (
    SELECT
        CASE
            WHEN score > 90 THEN struct('A', score, 'Excellent')
            ELSE struct('C', score, 'Average')
        END as result
    FROM Scores
);
```

### struct() as Function Argument

```sql
SELECT
    json_encode(struct(id, name, price)) as json_data
FROM Products;
```

### Named struct() (if implemented)

```sql
SELECT struct(
    action => 'BLOCK',
    confidence => 0.95,
    reason => 'Suspicious activity'
) as decision
FROM Events;

SELECT decision.action, decision.confidence
FROM ...;
```

## Implementation Order

1. Add StructValue and AttributeValue::Struct
2. Add serialization support
3. Implement StructFunctionExecutor
4. Register struct() in function registry
5. Add FieldAccessExpression to AST
6. Implement FieldAccessExpressionExecutor
7. Add parser support for field access
8. Write unit tests
9. Write integration tests
10. (Optional) Add named struct syntax

## Dependencies

- None for basic implementation
- JSON serialization for output

## Ambiguity: Field Access vs Qualified Names

`x.y` can mean:
- Table x, column y (qualified name)
- Variable x, field y (field access)

Resolution strategies:

A. Context-aware parsing:
   - If x is known table/alias → qualified name
   - Otherwise → field access

B. Different syntax:
   - `x.y` for qualified names
   - `x->y` or `x:y` for field access

C. Runtime resolution:
   - Try as qualified name first
   - Fall back to field access

Recommendation: Start with (A) since it matches SQL semantics. If parsing becomes complex, switch to (B).

## Estimated Effort

- AttributeValue::Struct: 2-3 hours
- StructFunctionExecutor: 2-3 hours
- FieldAccessExpressionExecutor: 2-3 hours
- Parser changes: 4-6 hours (field access syntax)
- Serialization: 2-3 hours
- Testing: 4-6 hours
- Total: 16-24 hours (3-4 days)

## Files to Modify/Create

- src/core/event/value.rs (add Struct variant)
- src/query_api/definition/attribute.rs (add STRUCT type)
- src/query_api/expression/mod.rs (add FieldAccessExpression)
- src/core/executor/function/struct_function_executor.rs (new)
- src/core/executor/field_access_expression_executor.rs (new)
- src/core/executor/mod.rs (register new executors)
- src/core/util/parser/expression_parser.rs (add parsing)
- src/sql_compiler/ (field access syntax)
- tests/ (test cases)
