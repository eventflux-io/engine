# CASE Expression Implementation

This document specifies the implementation of SQL CASE/WHEN/ELSE/END expressions in EventFlux.

## Syntax

```sql
CASE
    WHEN condition1 THEN result1
    WHEN condition2 THEN result2
    ...
    ELSE default_result
END
```

Also simple CASE:

```sql
CASE expression
    WHEN value1 THEN result1
    WHEN value2 THEN result2
    ELSE default_result
END
```

## Components to Implement

### 1. AST Node

Location: src/query_api/expression/

```rust
pub enum Expression {
    // existing variants...
    Case(CaseExpression),
}

pub struct CaseExpression {
    /// Optional operand for simple CASE
    pub operand: Option<Box<Expression>>,
    /// List of WHEN conditions and results
    pub when_clauses: Vec<WhenClause>,
    /// Optional ELSE result
    pub else_result: Option<Box<Expression>>,
}

pub struct WhenClause {
    /// Condition (or value for simple CASE)
    pub condition: Box<Expression>,
    /// Result if condition is true
    pub result: Box<Expression>,
}
```

### 2. Parser Support

Location: src/sql_compiler/

sqlparser-rs already parses CASE expressions. Check if EventFluxDialect passes them through correctly.

If using LALRPOP grammar, add:

```
CaseExpr: Expression = {
    "CASE" <clauses:WhenClause+> <else_result:("ELSE" <Expr>)?> "END" => {
        Expression::Case(CaseExpression {
            operand: None,
            when_clauses: clauses,
            else_result: else_result.map(Box::new),
        })
    },
    "CASE" <operand:Expr> <clauses:WhenClause+> <else_result:("ELSE" <Expr>)?> "END" => {
        Expression::Case(CaseExpression {
            operand: Some(Box::new(operand)),
            when_clauses: clauses,
            else_result: else_result.map(Box::new),
        })
    },
};

WhenClause: WhenClause = {
    "WHEN" <condition:Expr> "THEN" <result:Expr> => {
        WhenClause {
            condition: Box::new(condition),
            result: Box::new(result),
        }
    },
};
```

### 3. Expression Executor

Location: src/core/executor/case_expression_executor.rs

```rust
use crate::core::executor::ExpressionExecutor;
use crate::core::event::stream_event::StreamEvent;
use crate::core::event::value::AttributeValue;

pub struct CaseExpressionExecutor {
    /// Optional operand executor for simple CASE
    operand: Option<Box<dyn ExpressionExecutor>>,
    /// List of (condition, result) executors
    when_clauses: Vec<(Box<dyn ExpressionExecutor>, Box<dyn ExpressionExecutor>)>,
    /// Optional else executor
    else_executor: Option<Box<dyn ExpressionExecutor>>,
}

impl CaseExpressionExecutor {
    pub fn new(
        operand: Option<Box<dyn ExpressionExecutor>>,
        when_clauses: Vec<(Box<dyn ExpressionExecutor>, Box<dyn ExpressionExecutor>)>,
        else_executor: Option<Box<dyn ExpressionExecutor>>,
    ) -> Self {
        Self {
            operand,
            when_clauses,
            else_executor,
        }
    }
}

impl ExpressionExecutor for CaseExpressionExecutor {
    fn execute(&self, event: Option<&StreamEvent>) -> Option<AttributeValue> {
        match &self.operand {
            // Searched CASE (CASE WHEN condition THEN ...)
            None => {
                for (condition, result) in &self.when_clauses {
                    match condition.execute(event) {
                        Some(AttributeValue::Bool(true)) => {
                            return result.execute(event);
                        }
                        _ => continue,
                    }
                }
            }
            // Simple CASE (CASE expr WHEN value THEN ...)
            Some(operand_exec) => {
                let operand_value = operand_exec.execute(event)?;
                for (value_exec, result) in &self.when_clauses {
                    let when_value = value_exec.execute(event)?;
                    if operand_value == when_value {
                        return result.execute(event);
                    }
                }
            }
        }

        // No match, return ELSE or NULL
        match &self.else_executor {
            Some(else_exec) => else_exec.execute(event),
            None => Some(AttributeValue::Null),
        }
    }

    fn get_return_type(&self) -> crate::query_api::definition::attribute::Type {
        // Return type is the common type of all result branches
        // For now, use the first result's type
        if let Some((_, result)) = self.when_clauses.first() {
            result.get_return_type()
        } else if let Some(else_exec) = &self.else_executor {
            else_exec.get_return_type()
        } else {
            crate::query_api::definition::attribute::Type::OBJECT
        }
    }
}
```

### 4. Expression Parser Integration

Location: src/core/util/parser/expression_parser.rs

Add case to the expression parsing:

```rust
fn parse_expression(&self, expr: &Expression) -> Result<Box<dyn ExpressionExecutor>, String> {
    match expr {
        // existing cases...
        Expression::Case(case_expr) => self.parse_case_expression(case_expr),
    }
}

fn parse_case_expression(&self, case: &CaseExpression) -> Result<Box<dyn ExpressionExecutor>, String> {
    let operand = match &case.operand {
        Some(op) => Some(self.parse_expression(op)?),
        None => None,
    };

    let mut when_clauses = Vec::new();
    for clause in &case.when_clauses {
        let condition = self.parse_expression(&clause.condition)?;
        let result = self.parse_expression(&clause.result)?;
        when_clauses.push((condition, result));
    }

    let else_executor = match &case.else_result {
        Some(else_expr) => Some(self.parse_expression(else_expr)?),
        None => None,
    };

    Ok(Box::new(CaseExpressionExecutor::new(
        operand,
        when_clauses,
        else_executor,
    )))
}
```

### 5. Type Checking

All result branches (WHEN results and ELSE) should return compatible types. Options:

A. Strict: All must be same type
B. Coercion: Widen to common type (int -> double)
C. Dynamic: Return OBJECT type, check at runtime

Recommendation: Start with (C), add (B) later.

## Test Cases

### Basic Searched CASE

```sql
SELECT
    CASE
        WHEN score > 90 THEN 'A'
        WHEN score > 80 THEN 'B'
        WHEN score > 70 THEN 'C'
        ELSE 'F'
    END as grade
FROM Scores;
```

### Simple CASE

```sql
SELECT
    CASE status
        WHEN 'PENDING' THEN 'Waiting'
        WHEN 'APPROVED' THEN 'Done'
        WHEN 'REJECTED' THEN 'Failed'
        ELSE 'Unknown'
    END as status_text
FROM Orders;
```

### Nested CASE

```sql
SELECT
    CASE
        WHEN type = 'A' THEN
            CASE
                WHEN value > 100 THEN 'A-High'
                ELSE 'A-Low'
            END
        ELSE 'Other'
    END as category
FROM Items;
```

### CASE with Numeric Results

```sql
SELECT
    CASE
        WHEN priority = 'HIGH' THEN 1
        WHEN priority = 'MEDIUM' THEN 2
        ELSE 3
    END as priority_order
FROM Tasks;
```

### CASE without ELSE

```sql
SELECT
    CASE
        WHEN is_special THEN 'Special'
    END as tag
FROM Items;
-- Returns NULL when not matched
```

### CASE with Expressions in Results

```sql
SELECT
    CASE
        WHEN discount_type = 'PERCENT' THEN price * (1 - discount_value / 100)
        WHEN discount_type = 'FIXED' THEN price - discount_value
        ELSE price
    END as final_price
FROM Products;
```

### CASE in WHERE Clause

```sql
SELECT *
FROM Orders
WHERE CASE
    WHEN priority = 'HIGH' THEN amount > 1000
    WHEN priority = 'MEDIUM' THEN amount > 5000
    ELSE amount > 10000
END;
```

### CASE in ORDER BY

```sql
SELECT *
FROM Tasks
ORDER BY CASE
    WHEN status = 'URGENT' THEN 1
    WHEN status = 'NORMAL' THEN 2
    ELSE 3
END;
```

## Implementation Order

1. Add CaseExpression to query_api Expression enum
2. Implement CaseExpressionExecutor
3. Add parsing in expression_parser.rs
4. Write unit tests for executor
5. Write integration tests for full queries
6. Handle edge cases (NULL handling, type coercion)

## Dependencies

- None (uses existing expression framework)

## Estimated Effort

- Parser changes: 2-4 hours
- Executor implementation: 4-6 hours
- Expression parser integration: 2-3 hours
- Testing: 4-6 hours
- Total: 12-19 hours (2-3 days)

## Files to Modify

- src/query_api/expression/mod.rs (add CaseExpression)
- src/core/executor/mod.rs (add case_expression_executor)
- src/core/executor/case_expression_executor.rs (new file)
- src/core/util/parser/expression_parser.rs (add parsing)
- tests/ (add test cases)
