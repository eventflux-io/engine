// SPDX-License-Identifier: MIT OR Apache-2.0

// eventflux_rust/src/core/executor/function/mod.rs

pub mod builtin_wrapper;
pub mod cast_function_executor;
pub mod coalesce_function_executor;
pub mod convert_function_executor;
pub mod date_functions;
pub mod event_timestamp_function_executor;
pub mod instance_of_checkers;
pub mod math_functions;
pub mod scalar_function_executor; // Added
pub mod script_function_executor;
pub mod string_functions;
pub mod uuid_function_executor;

// pub mod function_executor_base; // If a base struct for stateful functions is created later

pub use self::builtin_wrapper::{BuiltinBuilder, BuiltinScalarFunction};
pub use self::cast_function_executor::CastFunctionExecutor;
pub use self::coalesce_function_executor::CoalesceFunctionExecutor;
pub use self::convert_function_executor::ConvertFunctionExecutor;
pub use self::date_functions::{
    CurrentTimestampFunctionExecutor, DateAddFunctionExecutor, FormatDateFunctionExecutor,
    ParseDateFunctionExecutor,
};
pub use self::event_timestamp_function_executor::EventTimestampFunctionExecutor;
pub use self::instance_of_checkers::*;
pub use self::math_functions::{
    LogFunctionExecutor, RoundFunctionExecutor, SinFunctionExecutor, SqrtFunctionExecutor,
    TanFunctionExecutor,
};
pub use self::scalar_function_executor::ScalarFunctionExecutor; // Added
pub use self::script_function_executor::ScriptFunctionExecutor;
pub use self::string_functions::{
    ConcatFunctionExecutor, LengthFunctionExecutor, LowerFunctionExecutor,
    SubstringFunctionExecutor, UpperFunctionExecutor,
};
pub use self::uuid_function_executor::UuidFunctionExecutor;

// Other function executors to be added here:
// pub mod cast_function_executor;
// pub mod convert_function_executor;
// pub mod script_function_executor;
// ... etc.
