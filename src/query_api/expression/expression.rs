// Corresponds to io.siddhi.query.api.expression.Expression (the abstract class)
// In Rust, this is represented as an enum.


// Import all the specific expression types
use super::attribute_function::AttributeFunction;
use super::condition::{And, Compare, CompareOperator, InOp, IsNull, Not, Or};
use super::constant::{Constant, TimeUtil as ConstantTimeUtil}; // Corrected ConstantValue path
use super::math::{Add, Divide, ModOp, Multiply, Subtract};
use super::variable::Variable; // Renamed Operator to CompareOperator

/// Represents various types of expressions in a Siddhi query.
#[derive(Clone, Debug, PartialEq)]
pub enum Expression {
    Constant(Constant),
    Variable(Variable),
    AttributeFunction(Box<AttributeFunction>),
    Add(Box<Add>),
    Subtract(Box<Subtract>),
    Multiply(Box<Multiply>),
    Divide(Box<Divide>),
    Mod(Box<ModOp>),
    And(Box<And>),
    Or(Box<Or>),
    Not(Box<Not>),
    Compare(Box<Compare>),
    In(Box<InOp>),
    IsNull(Box<IsNull>),
}

// Static factory methods from Java's Expression class
impl Expression {
    // Constants
    pub fn value_string(value: String) -> Self {
        Expression::Constant(Constant::string(value))
    }
    pub fn value_int(value: i32) -> Self {
        Expression::Constant(Constant::int(value))
    }
    pub fn value_long(value: i64) -> Self {
        Expression::Constant(Constant::long(value))
    }
    pub fn value_double(value: f64) -> Self {
        Expression::Constant(Constant::double(value))
    }
    pub fn value_float(value: f32) -> Self {
        Expression::Constant(Constant::float(value))
    }
    pub fn value_bool(value: bool) -> Self {
        Expression::Constant(Constant::bool(value))
    }

    // Variable
    pub fn variable(attribute_name: String) -> Self {
        Expression::Variable(Variable::new(attribute_name))
    }

    // AttributeFunction
    pub fn function(namespace: Option<String>, name: String, params: Vec<Expression>) -> Self {
        Expression::AttributeFunction(Box::new(AttributeFunction::new(namespace, name, params)))
    }
    pub fn function_no_ns(name: String, params: Vec<Expression>) -> Self {
        Expression::AttributeFunction(Box::new(AttributeFunction::new(None, name, params)))
    }

    // Math
    pub fn add(left: Expression, right: Expression) -> Self {
        Expression::Add(Box::new(Add::new(left, right)))
    }
    pub fn subtract(left: Expression, right: Expression) -> Self {
        Expression::Subtract(Box::new(Subtract::new(left, right)))
    }
    pub fn multiply(left: Expression, right: Expression) -> Self {
        Expression::Multiply(Box::new(Multiply::new(left, right)))
    }
    pub fn divide(left: Expression, right: Expression) -> Self {
        Expression::Divide(Box::new(Divide::new(left, right)))
    }
    pub fn modulus(left: Expression, right: Expression) -> Self {
        Expression::Mod(Box::new(ModOp::new(left, right)))
    }

    // Condition
    pub fn compare(left: Expression, op: CompareOperator, right: Expression) -> Self {
        Expression::Compare(Box::new(Compare::new(left, op, right)))
    }
    pub fn and(left: Expression, right: Expression) -> Self {
        Expression::And(Box::new(And::new(left, right)))
    }
    pub fn or(left: Expression, right: Expression) -> Self {
        Expression::Or(Box::new(Or::new(left, right)))
    }
    pub fn not(expr: Expression) -> Self {
        Expression::Not(Box::new(Not::new(expr)))
    }

    pub fn in_op(expr: Expression, source_id: String) -> Self {
        Expression::In(Box::new(InOp::new(expr, source_id)))
    }

    pub fn is_null(expr: Expression) -> Self {
        Expression::IsNull(Box::new(IsNull::new_with_expression(expr)))
    }
    pub fn is_null_stream(
        stream_id: String,
        stream_index: Option<i32>,
        is_inner: bool,
        is_fault: bool,
    ) -> Self {
        Expression::IsNull(Box::new(IsNull::new_with_stream_details(
            stream_id,
            stream_index,
            is_inner,
            is_fault,
        )))
    }

    // Time constants
    pub fn time_millisec(val: i64) -> Self {
        Expression::Constant(ConstantTimeUtil::millisec(val))
    }
    pub fn time_sec(val: i64) -> Self {
        Expression::Constant(ConstantTimeUtil::sec(val))
    }
    pub fn time_minute(val: i64) -> Self {
        Expression::Constant(ConstantTimeUtil::minute(val))
    }
    pub fn time_hour(val: i64) -> Self {
        Expression::Constant(ConstantTimeUtil::hour(val))
    }
    pub fn time_day(val: i64) -> Self {
        Expression::Constant(ConstantTimeUtil::day(val))
    }
    pub fn time_week(val: i64) -> Self {
        Expression::Constant(ConstantTimeUtil::week(val))
    }
    pub fn time_month(val: i64) -> Self {
        Expression::Constant(ConstantTimeUtil::month(val))
    }
    pub fn time_year(val: i64) -> Self {
        Expression::Constant(ConstantTimeUtil::year(val))
    }
}

// Accessing SiddhiElement context from the composed field in each variant's struct.
impl Expression {
    pub fn get_query_context_start_index(&self) -> Option<(i32, i32)> {
        match self {
            Expression::Constant(c) => c.siddhi_element.query_context_start_index,
            Expression::Variable(v) => v.siddhi_element.query_context_start_index,
            Expression::AttributeFunction(af) => af.siddhi_element.query_context_start_index,
            Expression::Add(a) => a.siddhi_element.query_context_start_index,
            Expression::Subtract(s) => s.siddhi_element.query_context_start_index,
            Expression::Multiply(m) => m.siddhi_element.query_context_start_index,
            Expression::Divide(d) => d.siddhi_element.query_context_start_index,
            Expression::Mod(m) => m.siddhi_element.query_context_start_index,
            Expression::And(a) => a.siddhi_element.query_context_start_index,
            Expression::Or(o) => o.siddhi_element.query_context_start_index,
            Expression::Not(n) => n.siddhi_element.query_context_start_index,
            Expression::Compare(c) => c.siddhi_element.query_context_start_index,
            Expression::In(i) => i.siddhi_element.query_context_start_index,
            Expression::IsNull(i) => i.siddhi_element.query_context_start_index,
        }
    }

    pub fn set_query_context_start_index(&mut self, index: Option<(i32, i32)>) {
        match self {
            Expression::Constant(c) => c.siddhi_element.query_context_start_index = index,
            Expression::Variable(v) => v.siddhi_element.query_context_start_index = index,
            Expression::AttributeFunction(af) => {
                af.siddhi_element.query_context_start_index = index
            }
            Expression::Add(a) => a.siddhi_element.query_context_start_index = index,
            Expression::Subtract(s) => s.siddhi_element.query_context_start_index = index,
            Expression::Multiply(m) => m.siddhi_element.query_context_start_index = index,
            Expression::Divide(d) => d.siddhi_element.query_context_start_index = index,
            Expression::Mod(m) => m.siddhi_element.query_context_start_index = index,
            Expression::And(a) => a.siddhi_element.query_context_start_index = index,
            Expression::Or(o) => o.siddhi_element.query_context_start_index = index,
            Expression::Not(n) => n.siddhi_element.query_context_start_index = index,
            Expression::Compare(c) => c.siddhi_element.query_context_start_index = index,
            Expression::In(i) => i.siddhi_element.query_context_start_index = index,
            Expression::IsNull(i) => i.siddhi_element.query_context_start_index = index,
        }
    }

    pub fn get_query_context_end_index(&self) -> Option<(i32, i32)> {
        match self {
            Expression::Constant(c) => c.siddhi_element.query_context_end_index,
            Expression::Variable(v) => v.siddhi_element.query_context_end_index,
            Expression::AttributeFunction(af) => af.siddhi_element.query_context_end_index,
            Expression::Add(a) => a.siddhi_element.query_context_end_index,
            Expression::Subtract(s) => s.siddhi_element.query_context_end_index,
            Expression::Multiply(m) => m.siddhi_element.query_context_end_index,
            Expression::Divide(d) => d.siddhi_element.query_context_end_index,
            Expression::Mod(m) => m.siddhi_element.query_context_end_index,
            Expression::And(a) => a.siddhi_element.query_context_end_index,
            Expression::Or(o) => o.siddhi_element.query_context_end_index,
            Expression::Not(n) => n.siddhi_element.query_context_end_index,
            Expression::Compare(c) => c.siddhi_element.query_context_end_index,
            Expression::In(i) => i.siddhi_element.query_context_end_index,
            Expression::IsNull(i) => i.siddhi_element.query_context_end_index,
        }
    }

    pub fn set_query_context_end_index(&mut self, index: Option<(i32, i32)>) {
        match self {
            Expression::Constant(c) => c.siddhi_element.query_context_end_index = index,
            Expression::Variable(v) => v.siddhi_element.query_context_end_index = index,
            Expression::AttributeFunction(af) => af.siddhi_element.query_context_end_index = index,
            Expression::Add(a) => a.siddhi_element.query_context_end_index = index,
            Expression::Subtract(s) => s.siddhi_element.query_context_end_index = index,
            Expression::Multiply(m) => m.siddhi_element.query_context_end_index = index,
            Expression::Divide(d) => d.siddhi_element.query_context_end_index = index,
            Expression::Mod(m) => m.siddhi_element.query_context_end_index = index,
            Expression::And(a) => a.siddhi_element.query_context_end_index = index,
            Expression::Or(o) => o.siddhi_element.query_context_end_index = index,
            Expression::Not(n) => n.siddhi_element.query_context_end_index = index,
            Expression::Compare(c) => c.siddhi_element.query_context_end_index = index,
            Expression::In(i) => i.siddhi_element.query_context_end_index = index,
            Expression::IsNull(i) => i.siddhi_element.query_context_end_index = index,
        }
    }
}
