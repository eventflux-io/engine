// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::core::config::eventflux_app_context::EventFluxAppContext;
use crate::core::event::complex_event::ComplexEvent;
use crate::core::event::value::AttributeValue;
use crate::core::executor::expression_executor::ExpressionExecutor;
use crate::core::executor::function::*;
use crate::query_api::definition::attribute::Type as ApiAttributeType;
use std::sync::Arc;

pub type BuiltinBuilder =
    fn(Vec<Box<dyn ExpressionExecutor>>) -> Result<Box<dyn ExpressionExecutor>, String>;

pub struct BuiltinScalarFunction {
    pub name: &'static str,
    pub builder: BuiltinBuilder,
    executor: Option<Box<dyn ExpressionExecutor>>,
}

impl Clone for BuiltinScalarFunction {
    fn clone(&self) -> Self {
        Self::new(self.name, self.builder)
    }
}

impl std::fmt::Debug for BuiltinScalarFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BuiltinScalarFunction")
            .field("name", &self.name)
            .finish()
    }
}

impl BuiltinScalarFunction {
    pub fn new(name: &'static str, builder: BuiltinBuilder) -> Self {
        Self {
            name,
            builder,
            executor: None,
        }
    }
}

impl ExpressionExecutor for BuiltinScalarFunction {
    fn execute(&self, event: Option<&dyn ComplexEvent>) -> Option<AttributeValue> {
        self.executor.as_ref()?.execute(event)
    }

    fn get_return_type(&self) -> ApiAttributeType {
        self.executor
            .as_ref()
            .map(|e| e.get_return_type())
            .unwrap_or(ApiAttributeType::OBJECT)
    }

    fn clone_executor(&self, ctx: &Arc<EventFluxAppContext>) -> Box<dyn ExpressionExecutor> {
        let mut cloned = Self::new(self.name, self.builder);
        if let Some(exec) = &self.executor {
            cloned.executor = Some(exec.clone_executor(ctx));
        }
        Box::new(cloned)
    }
}

impl ScalarFunctionExecutor for BuiltinScalarFunction {
    fn init(
        &mut self,
        args: &Vec<Box<dyn ExpressionExecutor>>,
        ctx: &Arc<EventFluxAppContext>,
    ) -> Result<(), String> {
        let cloned: Vec<Box<dyn ExpressionExecutor>> =
            args.iter().map(|e| e.clone_executor(ctx)).collect();
        let exec = (self.builder)(cloned)?;
        self.executor = Some(exec);
        Ok(())
    }

    fn destroy(&mut self) {
        self.executor = None;
    }

    fn get_name(&self) -> String {
        self.name.to_string()
    }

    fn clone_scalar_function(&self) -> Box<dyn ScalarFunctionExecutor> {
        Box::new(Self::new(self.name, self.builder))
    }
}

// --- Builtin builder helpers ---

fn build_cast(
    args: Vec<Box<dyn ExpressionExecutor>>,
) -> Result<Box<dyn ExpressionExecutor>, String> {
    if args.len() != 2 {
        return Err("cast() requires two arguments".to_string());
    }
    let mut a = args;
    let type_exec = a.remove(1);
    let val_exec = a.remove(0);
    Ok(Box::new(CastFunctionExecutor::new(val_exec, type_exec)?))
}

fn build_convert(
    args: Vec<Box<dyn ExpressionExecutor>>,
) -> Result<Box<dyn ExpressionExecutor>, String> {
    if args.len() != 2 {
        return Err("convert() requires two arguments".to_string());
    }
    let mut a = args;
    let type_exec = a.remove(1);
    let val_exec = a.remove(0);
    Ok(Box::new(ConvertFunctionExecutor::new(val_exec, type_exec)?))
}

fn build_concat(
    args: Vec<Box<dyn ExpressionExecutor>>,
) -> Result<Box<dyn ExpressionExecutor>, String> {
    Ok(Box::new(ConcatFunctionExecutor::new(args)?))
}

fn build_lower(
    args: Vec<Box<dyn ExpressionExecutor>>,
) -> Result<Box<dyn ExpressionExecutor>, String> {
    if args.len() != 1 {
        return Err("lower() requires one argument".to_string());
    }
    Ok(Box::new(LowerFunctionExecutor::new(
        args.into_iter().next().unwrap(),
    )?))
}

fn build_upper(
    args: Vec<Box<dyn ExpressionExecutor>>,
) -> Result<Box<dyn ExpressionExecutor>, String> {
    if args.len() != 1 {
        return Err("upper() requires one argument".to_string());
    }
    Ok(Box::new(UpperFunctionExecutor::new(
        args.into_iter().next().unwrap(),
    )?))
}

fn build_substring(
    mut args: Vec<Box<dyn ExpressionExecutor>>,
) -> Result<Box<dyn ExpressionExecutor>, String> {
    if args.len() < 2 || args.len() > 3 {
        return Err("substring() requires two or three arguments".to_string());
    }
    let length_exec = if args.len() == 3 {
        Some(args.remove(2))
    } else {
        None
    };
    let start_exec = args.remove(1);
    let val_exec = args.remove(0);
    Ok(Box::new(SubstringFunctionExecutor::new(
        val_exec,
        start_exec,
        length_exec,
    )?))
}

fn build_length(
    args: Vec<Box<dyn ExpressionExecutor>>,
) -> Result<Box<dyn ExpressionExecutor>, String> {
    if args.len() != 1 {
        return Err("length() requires one argument".to_string());
    }
    Ok(Box::new(LengthFunctionExecutor::new(
        args.into_iter().next().unwrap(),
    )?))
}

fn build_coalesce(
    args: Vec<Box<dyn ExpressionExecutor>>,
) -> Result<Box<dyn ExpressionExecutor>, String> {
    Ok(Box::new(CoalesceFunctionExecutor::new(args)?))
}

fn build_nullif(
    mut args: Vec<Box<dyn ExpressionExecutor>>,
) -> Result<Box<dyn ExpressionExecutor>, String> {
    if args.len() != 2 {
        return Err("nullif() requires exactly two arguments".to_string());
    }
    let second = args.remove(1);
    let first = args.remove(0);
    Ok(Box::new(NullIfFunctionExecutor::new(first, second)))
}

fn build_uuid(
    _args: Vec<Box<dyn ExpressionExecutor>>,
) -> Result<Box<dyn ExpressionExecutor>, String> {
    Ok(Box::new(UuidFunctionExecutor::new()))
}

fn build_current_timestamp(
    _args: Vec<Box<dyn ExpressionExecutor>>,
) -> Result<Box<dyn ExpressionExecutor>, String> {
    Ok(Box::new(CurrentTimestampFunctionExecutor))
}

fn build_format_date(
    mut args: Vec<Box<dyn ExpressionExecutor>>,
) -> Result<Box<dyn ExpressionExecutor>, String> {
    if args.len() != 2 {
        return Err("formatDate() requires two arguments".to_string());
    }
    let pattern = args.remove(1);
    let ts = args.remove(0);
    Ok(Box::new(FormatDateFunctionExecutor::new(ts, pattern)?))
}

fn build_parse_date(
    mut args: Vec<Box<dyn ExpressionExecutor>>,
) -> Result<Box<dyn ExpressionExecutor>, String> {
    if args.len() != 2 {
        return Err("parseDate() requires two arguments".to_string());
    }
    let pattern_exec = args.remove(1);
    let date_exec = args.remove(0);
    Ok(Box::new(ParseDateFunctionExecutor::new(
        date_exec,
        pattern_exec,
    )?))
}

fn build_date_add(
    mut args: Vec<Box<dyn ExpressionExecutor>>,
) -> Result<Box<dyn ExpressionExecutor>, String> {
    if args.len() != 3 {
        return Err("dateAdd() requires three arguments".to_string());
    }
    let unit_exec = args.remove(2);
    let inc_exec = args.remove(1);
    let ts_exec = args.remove(0);
    Ok(Box::new(DateAddFunctionExecutor::new(
        ts_exec, inc_exec, unit_exec,
    )?))
}

fn build_round(
    mut args: Vec<Box<dyn ExpressionExecutor>>,
) -> Result<Box<dyn ExpressionExecutor>, String> {
    match args.len() {
        1 => Ok(Box::new(RoundFunctionExecutor::new(args.remove(0))?)),
        2 => {
            let value_exec = args.remove(0);
            let precision_exec = args.remove(0);
            Ok(Box::new(RoundFunctionExecutor::new_with_precision(
                value_exec,
                precision_exec,
            )?))
        }
        _ => Err("round() requires one or two arguments".to_string()),
    }
}

fn build_sqrt(
    args: Vec<Box<dyn ExpressionExecutor>>,
) -> Result<Box<dyn ExpressionExecutor>, String> {
    if args.len() != 1 {
        return Err("sqrt() requires one argument".to_string());
    }
    Ok(Box::new(SqrtFunctionExecutor::new(
        args.into_iter().next().unwrap(),
    )?))
}

fn build_log(
    args: Vec<Box<dyn ExpressionExecutor>>,
) -> Result<Box<dyn ExpressionExecutor>, String> {
    if args.len() != 1 {
        return Err("log() requires one argument".to_string());
    }
    Ok(Box::new(LogFunctionExecutor::new(
        args.into_iter().next().unwrap(),
    )?))
}

fn build_sin(
    args: Vec<Box<dyn ExpressionExecutor>>,
) -> Result<Box<dyn ExpressionExecutor>, String> {
    if args.len() != 1 {
        return Err("sin() requires one argument".to_string());
    }
    Ok(Box::new(SinFunctionExecutor::new(
        args.into_iter().next().unwrap(),
    )?))
}

fn build_tan(
    args: Vec<Box<dyn ExpressionExecutor>>,
) -> Result<Box<dyn ExpressionExecutor>, String> {
    if args.len() != 1 {
        return Err("tan() requires one argument".to_string());
    }
    Ok(Box::new(TanFunctionExecutor::new(
        args.into_iter().next().unwrap(),
    )?))
}

fn build_abs(
    args: Vec<Box<dyn ExpressionExecutor>>,
) -> Result<Box<dyn ExpressionExecutor>, String> {
    if args.len() != 1 {
        return Err("abs() requires one argument".to_string());
    }
    Ok(Box::new(AbsFunctionExecutor::new(
        args.into_iter().next().unwrap(),
    )?))
}

fn build_floor(
    args: Vec<Box<dyn ExpressionExecutor>>,
) -> Result<Box<dyn ExpressionExecutor>, String> {
    if args.len() != 1 {
        return Err("floor() requires one argument".to_string());
    }
    Ok(Box::new(FloorFunctionExecutor::new(
        args.into_iter().next().unwrap(),
    )?))
}

fn build_ceil(
    args: Vec<Box<dyn ExpressionExecutor>>,
) -> Result<Box<dyn ExpressionExecutor>, String> {
    if args.len() != 1 {
        return Err("ceil() requires one argument".to_string());
    }
    Ok(Box::new(CeilFunctionExecutor::new(
        args.into_iter().next().unwrap(),
    )?))
}

fn build_cos(
    args: Vec<Box<dyn ExpressionExecutor>>,
) -> Result<Box<dyn ExpressionExecutor>, String> {
    if args.len() != 1 {
        return Err("cos() requires one argument".to_string());
    }
    Ok(Box::new(CosFunctionExecutor::new(
        args.into_iter().next().unwrap(),
    )?))
}

fn build_exp(
    args: Vec<Box<dyn ExpressionExecutor>>,
) -> Result<Box<dyn ExpressionExecutor>, String> {
    if args.len() != 1 {
        return Err("exp() requires one argument".to_string());
    }
    Ok(Box::new(ExpFunctionExecutor::new(
        args.into_iter().next().unwrap(),
    )?))
}

fn build_power(
    mut args: Vec<Box<dyn ExpressionExecutor>>,
) -> Result<Box<dyn ExpressionExecutor>, String> {
    if args.len() != 2 {
        return Err("power() requires two arguments".to_string());
    }
    let exponent = args.remove(1);
    let base = args.remove(0);
    Ok(Box::new(PowerFunctionExecutor::new(base, exponent)?))
}

fn build_trim(
    args: Vec<Box<dyn ExpressionExecutor>>,
) -> Result<Box<dyn ExpressionExecutor>, String> {
    if args.len() != 1 {
        return Err("trim() requires one argument".to_string());
    }
    Ok(Box::new(TrimFunctionExecutor::new(
        args.into_iter().next().unwrap(),
    )?))
}

fn build_like(
    mut args: Vec<Box<dyn ExpressionExecutor>>,
) -> Result<Box<dyn ExpressionExecutor>, String> {
    if args.len() != 2 {
        return Err("like() requires two arguments (value, pattern)".to_string());
    }
    let pattern = args.remove(1);
    let value = args.remove(0);
    Ok(Box::new(LikeFunctionExecutor::new(value, pattern)?))
}

fn build_replace(
    mut args: Vec<Box<dyn ExpressionExecutor>>,
) -> Result<Box<dyn ExpressionExecutor>, String> {
    if args.len() != 3 {
        return Err("replace() requires three arguments (value, from, to)".to_string());
    }
    let to = args.remove(2);
    let from = args.remove(1);
    let value = args.remove(0);
    Ok(Box::new(ReplaceFunctionExecutor::new(value, from, to)?))
}

fn build_log10(
    args: Vec<Box<dyn ExpressionExecutor>>,
) -> Result<Box<dyn ExpressionExecutor>, String> {
    if args.len() != 1 {
        return Err("log10() requires one argument".to_string());
    }
    Ok(Box::new(Log10FunctionExecutor::new(
        args.into_iter().next().unwrap(),
    )?))
}

fn build_event_timestamp(
    args: Vec<Box<dyn ExpressionExecutor>>,
) -> Result<Box<dyn ExpressionExecutor>, String> {
    if args.is_empty() {
        Ok(Box::new(EventTimestampFunctionExecutor::new(None)))
    } else if args.len() == 1 {
        Ok(Box::new(EventTimestampFunctionExecutor::new(Some(
            args.into_iter().next().unwrap(),
        ))))
    } else {
        Err("eventTimestamp() takes zero or one argument".to_string())
    }
}

/// Register default builtin scalar functions into the provided EventFluxContext.
pub fn register_builtin_scalar_functions(
    ctx: &crate::core::config::eventflux_context::EventFluxContext,
) {
    ctx.add_scalar_function_factory(
        "cast".to_string(),
        Box::new(BuiltinScalarFunction::new("cast", build_cast)),
    );
    ctx.add_scalar_function_factory(
        "convert".to_string(),
        Box::new(BuiltinScalarFunction::new("convert", build_convert)),
    );
    ctx.add_scalar_function_factory(
        "concat".to_string(),
        Box::new(BuiltinScalarFunction::new("concat", build_concat)),
    );
    ctx.add_scalar_function_factory(
        "str:concat".to_string(),
        Box::new(BuiltinScalarFunction::new("str:concat", build_concat)),
    );
    ctx.add_scalar_function_factory(
        "length".to_string(),
        Box::new(BuiltinScalarFunction::new("length", build_length)),
    );
    ctx.add_scalar_function_factory(
        "str:length".to_string(),
        Box::new(BuiltinScalarFunction::new("str:length", build_length)),
    );
    ctx.add_scalar_function_factory(
        "lower".to_string(),
        Box::new(BuiltinScalarFunction::new("lower", build_lower)),
    );
    ctx.add_scalar_function_factory(
        "str:lower".to_string(),
        Box::new(BuiltinScalarFunction::new("str:lower", build_lower)),
    );
    ctx.add_scalar_function_factory(
        "upper".to_string(),
        Box::new(BuiltinScalarFunction::new("upper", build_upper)),
    );
    ctx.add_scalar_function_factory(
        "str:upper".to_string(),
        Box::new(BuiltinScalarFunction::new("str:upper", build_upper)),
    );
    ctx.add_scalar_function_factory(
        "substring".to_string(),
        Box::new(BuiltinScalarFunction::new("substring", build_substring)),
    );
    ctx.add_scalar_function_factory(
        "str:substring".to_string(),
        Box::new(BuiltinScalarFunction::new("str:substring", build_substring)),
    );
    // substr is an alias for substring
    ctx.add_scalar_function_factory(
        "substr".to_string(),
        Box::new(BuiltinScalarFunction::new("substr", build_substring)),
    );
    ctx.add_scalar_function_factory(
        "str:substr".to_string(),
        Box::new(BuiltinScalarFunction::new("str:substr", build_substring)),
    );
    ctx.add_scalar_function_factory(
        "coalesce".to_string(),
        Box::new(BuiltinScalarFunction::new("coalesce", build_coalesce)),
    );
    ctx.add_scalar_function_factory(
        "nullif".to_string(),
        Box::new(BuiltinScalarFunction::new("nullif", build_nullif)),
    );
    ctx.add_scalar_function_factory(
        "uuid".to_string(),
        Box::new(BuiltinScalarFunction::new("uuid", build_uuid)),
    );
    ctx.add_scalar_function_factory(
        "currentTimestamp".to_string(),
        Box::new(BuiltinScalarFunction::new(
            "currentTimestamp",
            build_current_timestamp,
        )),
    );
    ctx.add_scalar_function_factory(
        "time:currentTimestamp".to_string(),
        Box::new(BuiltinScalarFunction::new(
            "time:currentTimestamp",
            build_current_timestamp,
        )),
    );
    ctx.add_scalar_function_factory(
        "formatDate".to_string(),
        Box::new(BuiltinScalarFunction::new("formatDate", build_format_date)),
    );
    ctx.add_scalar_function_factory(
        "time:formatDate".to_string(),
        Box::new(BuiltinScalarFunction::new(
            "time:formatDate",
            build_format_date,
        )),
    );
    ctx.add_scalar_function_factory(
        "parseDate".to_string(),
        Box::new(BuiltinScalarFunction::new("parseDate", build_parse_date)),
    );
    ctx.add_scalar_function_factory(
        "time:parseDate".to_string(),
        Box::new(BuiltinScalarFunction::new(
            "time:parseDate",
            build_parse_date,
        )),
    );
    ctx.add_scalar_function_factory(
        "dateAdd".to_string(),
        Box::new(BuiltinScalarFunction::new("dateAdd", build_date_add)),
    );
    ctx.add_scalar_function_factory(
        "time:dateAdd".to_string(),
        Box::new(BuiltinScalarFunction::new("time:dateAdd", build_date_add)),
    );
    ctx.add_scalar_function_factory(
        "round".to_string(),
        Box::new(BuiltinScalarFunction::new("round", build_round)),
    );
    ctx.add_scalar_function_factory(
        "math:round".to_string(),
        Box::new(BuiltinScalarFunction::new("math:round", build_round)),
    );
    ctx.add_scalar_function_factory(
        "sqrt".to_string(),
        Box::new(BuiltinScalarFunction::new("sqrt", build_sqrt)),
    );
    ctx.add_scalar_function_factory(
        "math:sqrt".to_string(),
        Box::new(BuiltinScalarFunction::new("math:sqrt", build_sqrt)),
    );
    ctx.add_scalar_function_factory(
        "log".to_string(),
        Box::new(BuiltinScalarFunction::new("log", build_log)),
    );
    ctx.add_scalar_function_factory(
        "math:log".to_string(),
        Box::new(BuiltinScalarFunction::new("math:log", build_log)),
    );
    ctx.add_scalar_function_factory(
        "sin".to_string(),
        Box::new(BuiltinScalarFunction::new("sin", build_sin)),
    );
    ctx.add_scalar_function_factory(
        "math:sin".to_string(),
        Box::new(BuiltinScalarFunction::new("math:sin", build_sin)),
    );
    ctx.add_scalar_function_factory(
        "tan".to_string(),
        Box::new(BuiltinScalarFunction::new("tan", build_tan)),
    );
    ctx.add_scalar_function_factory(
        "math:tan".to_string(),
        Box::new(BuiltinScalarFunction::new("math:tan", build_tan)),
    );
    ctx.add_scalar_function_factory(
        "eventTimestamp".to_string(),
        Box::new(BuiltinScalarFunction::new(
            "eventTimestamp",
            build_event_timestamp,
        )),
    );
    ctx.add_scalar_function_factory(
        "abs".to_string(),
        Box::new(BuiltinScalarFunction::new("abs", build_abs)),
    );
    ctx.add_scalar_function_factory(
        "math:abs".to_string(),
        Box::new(BuiltinScalarFunction::new("math:abs", build_abs)),
    );
    ctx.add_scalar_function_factory(
        "floor".to_string(),
        Box::new(BuiltinScalarFunction::new("floor", build_floor)),
    );
    ctx.add_scalar_function_factory(
        "math:floor".to_string(),
        Box::new(BuiltinScalarFunction::new("math:floor", build_floor)),
    );
    ctx.add_scalar_function_factory(
        "ceil".to_string(),
        Box::new(BuiltinScalarFunction::new("ceil", build_ceil)),
    );
    ctx.add_scalar_function_factory(
        "math:ceil".to_string(),
        Box::new(BuiltinScalarFunction::new("math:ceil", build_ceil)),
    );
    ctx.add_scalar_function_factory(
        "cos".to_string(),
        Box::new(BuiltinScalarFunction::new("cos", build_cos)),
    );
    ctx.add_scalar_function_factory(
        "math:cos".to_string(),
        Box::new(BuiltinScalarFunction::new("math:cos", build_cos)),
    );
    ctx.add_scalar_function_factory(
        "exp".to_string(),
        Box::new(BuiltinScalarFunction::new("exp", build_exp)),
    );
    ctx.add_scalar_function_factory(
        "math:exp".to_string(),
        Box::new(BuiltinScalarFunction::new("math:exp", build_exp)),
    );
    ctx.add_scalar_function_factory(
        "power".to_string(),
        Box::new(BuiltinScalarFunction::new("power", build_power)),
    );
    ctx.add_scalar_function_factory(
        "math:power".to_string(),
        Box::new(BuiltinScalarFunction::new("math:power", build_power)),
    );
    ctx.add_scalar_function_factory(
        "pow".to_string(),
        Box::new(BuiltinScalarFunction::new("pow", build_power)),
    );
    ctx.add_scalar_function_factory(
        "math:pow".to_string(),
        Box::new(BuiltinScalarFunction::new("math:pow", build_power)),
    );
    ctx.add_scalar_function_factory(
        "ln".to_string(),
        Box::new(BuiltinScalarFunction::new("ln", build_log)),
    );
    ctx.add_scalar_function_factory(
        "math:ln".to_string(),
        Box::new(BuiltinScalarFunction::new("math:ln", build_log)),
    );
    ctx.add_scalar_function_factory(
        "trim".to_string(),
        Box::new(BuiltinScalarFunction::new("trim", build_trim)),
    );
    ctx.add_scalar_function_factory(
        "str:trim".to_string(),
        Box::new(BuiltinScalarFunction::new("str:trim", build_trim)),
    );
    ctx.add_scalar_function_factory(
        "like".to_string(),
        Box::new(BuiltinScalarFunction::new("like", build_like)),
    );
    ctx.add_scalar_function_factory(
        "str:like".to_string(),
        Box::new(BuiltinScalarFunction::new("str:like", build_like)),
    );
    ctx.add_scalar_function_factory(
        "replace".to_string(),
        Box::new(BuiltinScalarFunction::new("replace", build_replace)),
    );
    ctx.add_scalar_function_factory(
        "str:replace".to_string(),
        Box::new(BuiltinScalarFunction::new("str:replace", build_replace)),
    );
    ctx.add_scalar_function_factory(
        "log10".to_string(),
        Box::new(BuiltinScalarFunction::new("log10", build_log10)),
    );
    ctx.add_scalar_function_factory(
        "math:log10".to_string(),
        Box::new(BuiltinScalarFunction::new("math:log10", build_log10)),
    );
}
