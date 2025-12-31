// SPDX-License-Identifier: MIT OR Apache-2.0

// eventflux_rust/src/core/executor/function/string_functions.rs
use crate::core::config::eventflux_app_context::EventFluxAppContext;
use crate::core::event::complex_event::ComplexEvent;
use crate::core::event::value::AttributeValue;
use crate::core::executor::expression_executor::ExpressionExecutor;
use crate::query_api::definition::attribute::Type as ApiAttributeType;
use std::sync::Arc;

#[derive(Debug)]
pub struct LengthFunctionExecutor {
    expr: Box<dyn ExpressionExecutor>,
}

impl LengthFunctionExecutor {
    pub fn new(expr: Box<dyn ExpressionExecutor>) -> Result<Self, String> {
        if expr.get_return_type() != ApiAttributeType::STRING {
            return Err("length() requires a STRING argument".to_string());
        }
        Ok(Self { expr })
    }
}

impl ExpressionExecutor for LengthFunctionExecutor {
    fn execute(&self, event: Option<&dyn ComplexEvent>) -> Option<AttributeValue> {
        match self.expr.execute(event)? {
            AttributeValue::String(s) => Some(AttributeValue::Int(s.len() as i32)),
            AttributeValue::Null => Some(AttributeValue::Null),
            _ => None,
        }
    }

    fn get_return_type(&self) -> ApiAttributeType {
        ApiAttributeType::INT
    }

    fn clone_executor(&self, ctx: &Arc<EventFluxAppContext>) -> Box<dyn ExpressionExecutor> {
        Box::new(LengthFunctionExecutor {
            expr: self.expr.clone_executor(ctx),
        })
    }
}

#[derive(Debug)]
pub struct ConcatFunctionExecutor {
    executors: Vec<Box<dyn ExpressionExecutor>>,
}

impl ConcatFunctionExecutor {
    pub fn new(executors: Vec<Box<dyn ExpressionExecutor>>) -> Result<Self, String> {
        if executors.is_empty() {
            return Err("concat() requires at least one argument".to_string());
        }
        for e in &executors {
            if e.get_return_type() != ApiAttributeType::STRING {
                return Err("concat() arguments must be STRING".to_string());
            }
        }
        Ok(Self { executors })
    }
}

impl ExpressionExecutor for ConcatFunctionExecutor {
    fn execute(&self, event: Option<&dyn ComplexEvent>) -> Option<AttributeValue> {
        let mut result = String::new();
        for e in &self.executors {
            match e.execute(event)? {
                AttributeValue::String(s) => result.push_str(&s),
                AttributeValue::Null => return Some(AttributeValue::Null),
                _ => return None,
            }
        }
        Some(AttributeValue::String(result))
    }

    fn get_return_type(&self) -> ApiAttributeType {
        ApiAttributeType::STRING
    }

    fn clone_executor(&self, ctx: &Arc<EventFluxAppContext>) -> Box<dyn ExpressionExecutor> {
        Box::new(ConcatFunctionExecutor {
            executors: self
                .executors
                .iter()
                .map(|e| e.clone_executor(ctx))
                .collect(),
        })
    }
}

fn to_i32(val: &AttributeValue) -> Option<i32> {
    match val {
        AttributeValue::Int(v) => Some(*v),
        AttributeValue::Long(v) => Some(*v as i32),
        AttributeValue::Float(v) => Some(*v as i32),
        AttributeValue::Double(v) => Some(*v as i32),
        _ => None,
    }
}

#[derive(Debug)]
pub struct LowerFunctionExecutor {
    expr: Box<dyn ExpressionExecutor>,
}

impl LowerFunctionExecutor {
    pub fn new(expr: Box<dyn ExpressionExecutor>) -> Result<Self, String> {
        if expr.get_return_type() != ApiAttributeType::STRING {
            return Err("lower() requires a STRING argument".to_string());
        }
        Ok(Self { expr })
    }
}

impl ExpressionExecutor for LowerFunctionExecutor {
    fn execute(&self, event: Option<&dyn ComplexEvent>) -> Option<AttributeValue> {
        match self.expr.execute(event)? {
            AttributeValue::String(s) => Some(AttributeValue::String(s.to_lowercase())),
            AttributeValue::Null => Some(AttributeValue::Null),
            _ => None,
        }
    }

    fn get_return_type(&self) -> ApiAttributeType {
        ApiAttributeType::STRING
    }

    fn clone_executor(&self, ctx: &Arc<EventFluxAppContext>) -> Box<dyn ExpressionExecutor> {
        Box::new(LowerFunctionExecutor {
            expr: self.expr.clone_executor(ctx),
        })
    }
}

#[derive(Debug)]
pub struct UpperFunctionExecutor {
    expr: Box<dyn ExpressionExecutor>,
}

impl UpperFunctionExecutor {
    pub fn new(expr: Box<dyn ExpressionExecutor>) -> Result<Self, String> {
        if expr.get_return_type() != ApiAttributeType::STRING {
            return Err("upper() requires a STRING argument".to_string());
        }
        Ok(Self { expr })
    }
}

impl ExpressionExecutor for UpperFunctionExecutor {
    fn execute(&self, event: Option<&dyn ComplexEvent>) -> Option<AttributeValue> {
        match self.expr.execute(event)? {
            AttributeValue::String(s) => Some(AttributeValue::String(s.to_uppercase())),
            AttributeValue::Null => Some(AttributeValue::Null),
            _ => None,
        }
    }

    fn get_return_type(&self) -> ApiAttributeType {
        ApiAttributeType::STRING
    }

    fn clone_executor(&self, ctx: &Arc<EventFluxAppContext>) -> Box<dyn ExpressionExecutor> {
        Box::new(UpperFunctionExecutor {
            expr: self.expr.clone_executor(ctx),
        })
    }
}

#[derive(Debug)]
pub struct SubstringFunctionExecutor {
    value_executor: Box<dyn ExpressionExecutor>,
    start_executor: Box<dyn ExpressionExecutor>,
    length_executor: Option<Box<dyn ExpressionExecutor>>,
}

impl SubstringFunctionExecutor {
    pub fn new(
        value_executor: Box<dyn ExpressionExecutor>,
        start_executor: Box<dyn ExpressionExecutor>,
        length_executor: Option<Box<dyn ExpressionExecutor>>,
    ) -> Result<Self, String> {
        if value_executor.get_return_type() != ApiAttributeType::STRING {
            return Err("substring() requires STRING as first argument".to_string());
        }
        if start_executor.get_return_type() == ApiAttributeType::STRING {
            return Err("substring() start index must be numeric".to_string());
        }
        if let Some(le) = &length_executor {
            if le.get_return_type() == ApiAttributeType::STRING {
                return Err("substring() length must be numeric".to_string());
            }
        }
        Ok(Self {
            value_executor,
            start_executor,
            length_executor,
        })
    }
}

impl ExpressionExecutor for SubstringFunctionExecutor {
    fn execute(&self, event: Option<&dyn ComplexEvent>) -> Option<AttributeValue> {
        let value = self.value_executor.execute(event)?;
        let s = match value {
            AttributeValue::String(v) => v,
            AttributeValue::Null => return Some(AttributeValue::Null),
            _ => return None,
        };

        let start_val = self.start_executor.execute(event)?;
        let start_idx = to_i32(&start_val)?;
        // Use 0-based indexing (Rust native) - SQL 1-based conversion happens at converter level
        let start = if start_idx < 0 { 0 } else { start_idx as usize };

        let substr = if let Some(le) = &self.length_executor {
            let len_val = le.execute(event)?;
            let len = to_i32(&len_val)? as usize;
            if start >= s.len() {
                String::new()
            } else {
                let end = usize::min(start + len, s.len());
                s[start..end].to_string()
            }
        } else if start >= s.len() {
            String::new()
        } else {
            s[start..].to_string()
        };

        Some(AttributeValue::String(substr))
    }

    fn get_return_type(&self) -> ApiAttributeType {
        ApiAttributeType::STRING
    }

    fn clone_executor(&self, ctx: &Arc<EventFluxAppContext>) -> Box<dyn ExpressionExecutor> {
        Box::new(SubstringFunctionExecutor {
            value_executor: self.value_executor.clone_executor(ctx),
            start_executor: self.start_executor.clone_executor(ctx),
            length_executor: self.length_executor.as_ref().map(|e| e.clone_executor(ctx)),
        })
    }
}

#[derive(Debug)]
pub struct TrimFunctionExecutor {
    expr: Box<dyn ExpressionExecutor>,
}

impl TrimFunctionExecutor {
    pub fn new(expr: Box<dyn ExpressionExecutor>) -> Result<Self, String> {
        Ok(Self { expr })
    }
}

impl ExpressionExecutor for TrimFunctionExecutor {
    fn execute(&self, event: Option<&dyn ComplexEvent>) -> Option<AttributeValue> {
        match self.expr.execute(event)? {
            AttributeValue::String(s) => Some(AttributeValue::String(s.trim().to_string())),
            AttributeValue::Null => Some(AttributeValue::Null),
            _ => None,
        }
    }

    fn get_return_type(&self) -> ApiAttributeType {
        ApiAttributeType::STRING
    }

    fn clone_executor(&self, ctx: &Arc<EventFluxAppContext>) -> Box<dyn ExpressionExecutor> {
        Box::new(TrimFunctionExecutor {
            expr: self.expr.clone_executor(ctx),
        })
    }
}

#[derive(Debug)]
pub struct LikeFunctionExecutor {
    value_executor: Box<dyn ExpressionExecutor>,
    pattern_executor: Box<dyn ExpressionExecutor>,
}

impl LikeFunctionExecutor {
    pub fn new(
        value_executor: Box<dyn ExpressionExecutor>,
        pattern_executor: Box<dyn ExpressionExecutor>,
    ) -> Result<Self, String> {
        Ok(Self {
            value_executor,
            pattern_executor,
        })
    }

    /// Convert SQL LIKE pattern to regex pattern
    /// % matches any sequence of characters
    /// _ matches any single character
    fn like_to_regex(pattern: &str) -> String {
        let mut regex = String::from("^");
        let mut chars = pattern.chars().peekable();

        while let Some(c) = chars.next() {
            match c {
                '%' => regex.push_str(".*"),
                '_' => regex.push('.'),
                '\\' => {
                    // Escape next character
                    if let Some(&next) = chars.peek() {
                        chars.next();
                        regex.push_str(&regex::escape(&next.to_string()));
                    }
                }
                _ => regex.push_str(&regex::escape(&c.to_string())),
            }
        }

        regex.push('$');
        regex
    }
}

impl ExpressionExecutor for LikeFunctionExecutor {
    fn execute(&self, event: Option<&dyn ComplexEvent>) -> Option<AttributeValue> {
        let value = self.value_executor.execute(event)?;
        let pattern = self.pattern_executor.execute(event)?;

        match (&value, &pattern) {
            (AttributeValue::Null, _) | (_, AttributeValue::Null) => Some(AttributeValue::Null),
            (AttributeValue::String(s), AttributeValue::String(p)) => {
                let regex_pattern = Self::like_to_regex(p);
                match regex::Regex::new(&regex_pattern) {
                    Ok(re) => Some(AttributeValue::Bool(re.is_match(s))),
                    Err(_) => None,
                }
            }
            _ => None,
        }
    }

    fn get_return_type(&self) -> ApiAttributeType {
        ApiAttributeType::BOOL
    }

    fn clone_executor(&self, ctx: &Arc<EventFluxAppContext>) -> Box<dyn ExpressionExecutor> {
        Box::new(LikeFunctionExecutor {
            value_executor: self.value_executor.clone_executor(ctx),
            pattern_executor: self.pattern_executor.clone_executor(ctx),
        })
    }
}

#[derive(Debug)]
pub struct ReplaceFunctionExecutor {
    value_executor: Box<dyn ExpressionExecutor>,
    from_executor: Box<dyn ExpressionExecutor>,
    to_executor: Box<dyn ExpressionExecutor>,
}

impl ReplaceFunctionExecutor {
    pub fn new(
        value_executor: Box<dyn ExpressionExecutor>,
        from_executor: Box<dyn ExpressionExecutor>,
        to_executor: Box<dyn ExpressionExecutor>,
    ) -> Result<Self, String> {
        Ok(Self {
            value_executor,
            from_executor,
            to_executor,
        })
    }
}

impl ExpressionExecutor for ReplaceFunctionExecutor {
    fn execute(&self, event: Option<&dyn ComplexEvent>) -> Option<AttributeValue> {
        let value = self.value_executor.execute(event)?;
        let from = self.from_executor.execute(event)?;
        let to = self.to_executor.execute(event)?;

        match (&value, &from, &to) {
            (AttributeValue::Null, _, _)
            | (_, AttributeValue::Null, _)
            | (_, _, AttributeValue::Null) => Some(AttributeValue::Null),
            (AttributeValue::String(s), AttributeValue::String(f), AttributeValue::String(t)) => {
                Some(AttributeValue::String(s.replace(f.as_str(), t.as_str())))
            }
            _ => None,
        }
    }

    fn get_return_type(&self) -> ApiAttributeType {
        ApiAttributeType::STRING
    }

    fn clone_executor(&self, ctx: &Arc<EventFluxAppContext>) -> Box<dyn ExpressionExecutor> {
        Box::new(ReplaceFunctionExecutor {
            value_executor: self.value_executor.clone_executor(ctx),
            from_executor: self.from_executor.clone_executor(ctx),
            to_executor: self.to_executor.clone_executor(ctx),
        })
    }
}
