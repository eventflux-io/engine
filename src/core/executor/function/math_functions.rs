// SPDX-License-Identifier: MIT OR Apache-2.0

// eventflux_rust/src/core/executor/function/math_functions.rs
use crate::core::config::eventflux_app_context::EventFluxAppContext;
use crate::core::event::complex_event::ComplexEvent;
use crate::core::event::value::AttributeValue;
use crate::core::executor::expression_executor::ExpressionExecutor;
use crate::query_api::definition::attribute::Type as ApiAttributeType;
use std::sync::Arc;

fn to_f64(val: &AttributeValue) -> Option<f64> {
    match val {
        AttributeValue::Int(v) => Some(*v as f64),
        AttributeValue::Long(v) => Some(*v as f64),
        AttributeValue::Float(v) => Some(*v as f64),
        AttributeValue::Double(v) => Some(*v),
        _ => None,
    }
}

#[derive(Debug)]
pub struct SqrtFunctionExecutor {
    value_executor: Box<dyn ExpressionExecutor>,
}

impl SqrtFunctionExecutor {
    pub fn new(value_executor: Box<dyn ExpressionExecutor>) -> Result<Self, String> {
        Ok(Self { value_executor })
    }
}

impl ExpressionExecutor for SqrtFunctionExecutor {
    fn execute(&self, event: Option<&dyn ComplexEvent>) -> Option<AttributeValue> {
        let val = self.value_executor.execute(event)?;
        match val {
            AttributeValue::Null => Some(AttributeValue::Null),
            _ => {
                let num = to_f64(&val)?;
                Some(AttributeValue::Double(num.sqrt()))
            }
        }
    }

    fn get_return_type(&self) -> ApiAttributeType {
        ApiAttributeType::DOUBLE
    }

    fn clone_executor(&self, ctx: &Arc<EventFluxAppContext>) -> Box<dyn ExpressionExecutor> {
        Box::new(SqrtFunctionExecutor {
            value_executor: self.value_executor.clone_executor(ctx),
        })
    }
}

#[derive(Debug)]
pub struct RoundFunctionExecutor {
    value_executor: Box<dyn ExpressionExecutor>,
    precision_executor: Option<Box<dyn ExpressionExecutor>>,
}

impl RoundFunctionExecutor {
    pub fn new(value_executor: Box<dyn ExpressionExecutor>) -> Result<Self, String> {
        Ok(Self {
            value_executor,
            precision_executor: None,
        })
    }

    pub fn new_with_precision(
        value_executor: Box<dyn ExpressionExecutor>,
        precision_executor: Box<dyn ExpressionExecutor>,
    ) -> Result<Self, String> {
        Ok(Self {
            value_executor,
            precision_executor: Some(precision_executor),
        })
    }
}

#[derive(Debug)]
pub struct LogFunctionExecutor {
    value_executor: Box<dyn ExpressionExecutor>,
}

impl LogFunctionExecutor {
    pub fn new(value_executor: Box<dyn ExpressionExecutor>) -> Result<Self, String> {
        Ok(Self { value_executor })
    }
}

impl ExpressionExecutor for LogFunctionExecutor {
    fn execute(&self, event: Option<&dyn ComplexEvent>) -> Option<AttributeValue> {
        let val = self.value_executor.execute(event)?;
        match val {
            AttributeValue::Null => Some(AttributeValue::Null),
            _ => {
                let num = to_f64(&val)?;
                Some(AttributeValue::Double(num.ln()))
            }
        }
    }

    fn get_return_type(&self) -> ApiAttributeType {
        ApiAttributeType::DOUBLE
    }

    fn clone_executor(&self, ctx: &Arc<EventFluxAppContext>) -> Box<dyn ExpressionExecutor> {
        Box::new(LogFunctionExecutor {
            value_executor: self.value_executor.clone_executor(ctx),
        })
    }
}

#[derive(Debug)]
pub struct SinFunctionExecutor {
    value_executor: Box<dyn ExpressionExecutor>,
}

impl SinFunctionExecutor {
    pub fn new(value_executor: Box<dyn ExpressionExecutor>) -> Result<Self, String> {
        Ok(Self { value_executor })
    }
}

impl ExpressionExecutor for SinFunctionExecutor {
    fn execute(&self, event: Option<&dyn ComplexEvent>) -> Option<AttributeValue> {
        let val = self.value_executor.execute(event)?;
        match val {
            AttributeValue::Null => Some(AttributeValue::Null),
            _ => {
                let num = to_f64(&val)?;
                Some(AttributeValue::Double(num.sin()))
            }
        }
    }

    fn get_return_type(&self) -> ApiAttributeType {
        ApiAttributeType::DOUBLE
    }

    fn clone_executor(&self, ctx: &Arc<EventFluxAppContext>) -> Box<dyn ExpressionExecutor> {
        Box::new(SinFunctionExecutor {
            value_executor: self.value_executor.clone_executor(ctx),
        })
    }
}

#[derive(Debug)]
pub struct TanFunctionExecutor {
    value_executor: Box<dyn ExpressionExecutor>,
}

impl TanFunctionExecutor {
    pub fn new(value_executor: Box<dyn ExpressionExecutor>) -> Result<Self, String> {
        Ok(Self { value_executor })
    }
}

impl ExpressionExecutor for TanFunctionExecutor {
    fn execute(&self, event: Option<&dyn ComplexEvent>) -> Option<AttributeValue> {
        let val = self.value_executor.execute(event)?;
        match val {
            AttributeValue::Null => Some(AttributeValue::Null),
            _ => {
                let num = to_f64(&val)?;
                Some(AttributeValue::Double(num.tan()))
            }
        }
    }

    fn get_return_type(&self) -> ApiAttributeType {
        ApiAttributeType::DOUBLE
    }

    fn clone_executor(&self, ctx: &Arc<EventFluxAppContext>) -> Box<dyn ExpressionExecutor> {
        Box::new(TanFunctionExecutor {
            value_executor: self.value_executor.clone_executor(ctx),
        })
    }
}

impl ExpressionExecutor for RoundFunctionExecutor {
    fn execute(&self, event: Option<&dyn ComplexEvent>) -> Option<AttributeValue> {
        let val = self.value_executor.execute(event)?;
        match val {
            AttributeValue::Null => Some(AttributeValue::Null),
            _ => {
                let num = to_f64(&val)?;
                let result = if let Some(ref precision_exec) = self.precision_executor {
                    let precision_val = precision_exec.execute(event)?;
                    let precision = match precision_val {
                        AttributeValue::Int(p) => p,
                        AttributeValue::Long(p) => p as i32,
                        _ => 0,
                    };
                    let multiplier = 10_f64.powi(precision);
                    (num * multiplier).round() / multiplier
                } else {
                    num.round()
                };
                Some(AttributeValue::Double(result))
            }
        }
    }

    fn get_return_type(&self) -> ApiAttributeType {
        ApiAttributeType::DOUBLE
    }

    fn clone_executor(&self, ctx: &Arc<EventFluxAppContext>) -> Box<dyn ExpressionExecutor> {
        Box::new(RoundFunctionExecutor {
            value_executor: self.value_executor.clone_executor(ctx),
            precision_executor: self
                .precision_executor
                .as_ref()
                .map(|e| e.clone_executor(ctx)),
        })
    }
}

#[derive(Debug)]
pub struct AbsFunctionExecutor {
    value_executor: Box<dyn ExpressionExecutor>,
}

impl AbsFunctionExecutor {
    pub fn new(value_executor: Box<dyn ExpressionExecutor>) -> Result<Self, String> {
        Ok(Self { value_executor })
    }
}

impl ExpressionExecutor for AbsFunctionExecutor {
    fn execute(&self, event: Option<&dyn ComplexEvent>) -> Option<AttributeValue> {
        let val = self.value_executor.execute(event)?;
        match val {
            AttributeValue::Null => Some(AttributeValue::Null),
            AttributeValue::Int(v) => Some(AttributeValue::Int(v.abs())),
            AttributeValue::Long(v) => Some(AttributeValue::Long(v.abs())),
            AttributeValue::Float(v) => Some(AttributeValue::Float(v.abs())),
            AttributeValue::Double(v) => Some(AttributeValue::Double(v.abs())),
            _ => None,
        }
    }

    fn get_return_type(&self) -> ApiAttributeType {
        self.value_executor.get_return_type()
    }

    fn clone_executor(&self, ctx: &Arc<EventFluxAppContext>) -> Box<dyn ExpressionExecutor> {
        Box::new(AbsFunctionExecutor {
            value_executor: self.value_executor.clone_executor(ctx),
        })
    }
}

#[derive(Debug)]
pub struct FloorFunctionExecutor {
    value_executor: Box<dyn ExpressionExecutor>,
}

impl FloorFunctionExecutor {
    pub fn new(value_executor: Box<dyn ExpressionExecutor>) -> Result<Self, String> {
        Ok(Self { value_executor })
    }
}

impl ExpressionExecutor for FloorFunctionExecutor {
    fn execute(&self, event: Option<&dyn ComplexEvent>) -> Option<AttributeValue> {
        let val = self.value_executor.execute(event)?;
        match val {
            AttributeValue::Null => Some(AttributeValue::Null),
            _ => {
                let num = to_f64(&val)?;
                Some(AttributeValue::Double(num.floor()))
            }
        }
    }

    fn get_return_type(&self) -> ApiAttributeType {
        ApiAttributeType::DOUBLE
    }

    fn clone_executor(&self, ctx: &Arc<EventFluxAppContext>) -> Box<dyn ExpressionExecutor> {
        Box::new(FloorFunctionExecutor {
            value_executor: self.value_executor.clone_executor(ctx),
        })
    }
}

#[derive(Debug)]
pub struct CeilFunctionExecutor {
    value_executor: Box<dyn ExpressionExecutor>,
}

impl CeilFunctionExecutor {
    pub fn new(value_executor: Box<dyn ExpressionExecutor>) -> Result<Self, String> {
        Ok(Self { value_executor })
    }
}

impl ExpressionExecutor for CeilFunctionExecutor {
    fn execute(&self, event: Option<&dyn ComplexEvent>) -> Option<AttributeValue> {
        let val = self.value_executor.execute(event)?;
        match val {
            AttributeValue::Null => Some(AttributeValue::Null),
            _ => {
                let num = to_f64(&val)?;
                Some(AttributeValue::Double(num.ceil()))
            }
        }
    }

    fn get_return_type(&self) -> ApiAttributeType {
        ApiAttributeType::DOUBLE
    }

    fn clone_executor(&self, ctx: &Arc<EventFluxAppContext>) -> Box<dyn ExpressionExecutor> {
        Box::new(CeilFunctionExecutor {
            value_executor: self.value_executor.clone_executor(ctx),
        })
    }
}

#[derive(Debug)]
pub struct CosFunctionExecutor {
    value_executor: Box<dyn ExpressionExecutor>,
}

impl CosFunctionExecutor {
    pub fn new(value_executor: Box<dyn ExpressionExecutor>) -> Result<Self, String> {
        Ok(Self { value_executor })
    }
}

impl ExpressionExecutor for CosFunctionExecutor {
    fn execute(&self, event: Option<&dyn ComplexEvent>) -> Option<AttributeValue> {
        let val = self.value_executor.execute(event)?;
        match val {
            AttributeValue::Null => Some(AttributeValue::Null),
            _ => {
                let num = to_f64(&val)?;
                Some(AttributeValue::Double(num.cos()))
            }
        }
    }

    fn get_return_type(&self) -> ApiAttributeType {
        ApiAttributeType::DOUBLE
    }

    fn clone_executor(&self, ctx: &Arc<EventFluxAppContext>) -> Box<dyn ExpressionExecutor> {
        Box::new(CosFunctionExecutor {
            value_executor: self.value_executor.clone_executor(ctx),
        })
    }
}

#[derive(Debug)]
pub struct ExpFunctionExecutor {
    value_executor: Box<dyn ExpressionExecutor>,
}

impl ExpFunctionExecutor {
    pub fn new(value_executor: Box<dyn ExpressionExecutor>) -> Result<Self, String> {
        Ok(Self { value_executor })
    }
}

impl ExpressionExecutor for ExpFunctionExecutor {
    fn execute(&self, event: Option<&dyn ComplexEvent>) -> Option<AttributeValue> {
        let val = self.value_executor.execute(event)?;
        match val {
            AttributeValue::Null => Some(AttributeValue::Null),
            _ => {
                let num = to_f64(&val)?;
                Some(AttributeValue::Double(num.exp()))
            }
        }
    }

    fn get_return_type(&self) -> ApiAttributeType {
        ApiAttributeType::DOUBLE
    }

    fn clone_executor(&self, ctx: &Arc<EventFluxAppContext>) -> Box<dyn ExpressionExecutor> {
        Box::new(ExpFunctionExecutor {
            value_executor: self.value_executor.clone_executor(ctx),
        })
    }
}

#[derive(Debug)]
pub struct PowerFunctionExecutor {
    base_executor: Box<dyn ExpressionExecutor>,
    exponent_executor: Box<dyn ExpressionExecutor>,
}

impl PowerFunctionExecutor {
    pub fn new(
        base_executor: Box<dyn ExpressionExecutor>,
        exponent_executor: Box<dyn ExpressionExecutor>,
    ) -> Result<Self, String> {
        Ok(Self {
            base_executor,
            exponent_executor,
        })
    }
}

impl ExpressionExecutor for PowerFunctionExecutor {
    fn execute(&self, event: Option<&dyn ComplexEvent>) -> Option<AttributeValue> {
        let base_val = self.base_executor.execute(event)?;
        let exp_val = self.exponent_executor.execute(event)?;
        match (&base_val, &exp_val) {
            (AttributeValue::Null, _) | (_, AttributeValue::Null) => Some(AttributeValue::Null),
            _ => {
                let base = to_f64(&base_val)?;
                let exp = to_f64(&exp_val)?;
                Some(AttributeValue::Double(base.powf(exp)))
            }
        }
    }

    fn get_return_type(&self) -> ApiAttributeType {
        ApiAttributeType::DOUBLE
    }

    fn clone_executor(&self, ctx: &Arc<EventFluxAppContext>) -> Box<dyn ExpressionExecutor> {
        Box::new(PowerFunctionExecutor {
            base_executor: self.base_executor.clone_executor(ctx),
            exponent_executor: self.exponent_executor.clone_executor(ctx),
        })
    }
}

#[derive(Debug)]
pub struct Log10FunctionExecutor {
    value_executor: Box<dyn ExpressionExecutor>,
}

impl Log10FunctionExecutor {
    pub fn new(value_executor: Box<dyn ExpressionExecutor>) -> Result<Self, String> {
        Ok(Self { value_executor })
    }
}

impl ExpressionExecutor for Log10FunctionExecutor {
    fn execute(&self, event: Option<&dyn ComplexEvent>) -> Option<AttributeValue> {
        let val = self.value_executor.execute(event)?;
        match val {
            AttributeValue::Null => Some(AttributeValue::Null),
            _ => {
                let num = to_f64(&val)?;
                Some(AttributeValue::Double(num.log10()))
            }
        }
    }

    fn get_return_type(&self) -> ApiAttributeType {
        ApiAttributeType::DOUBLE
    }

    fn clone_executor(&self, ctx: &Arc<EventFluxAppContext>) -> Box<dyn ExpressionExecutor> {
        Box::new(Log10FunctionExecutor {
            value_executor: self.value_executor.clone_executor(ctx),
        })
    }
}
