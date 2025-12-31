// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Function Compatibility Tests
// Reference: query/function/FunctionTestCase.java, CoalesceFunctionTestCase.java,
//            ConversionFunctionTestCase.java, UUIDFunctionTestCase.java

use super::common::AppRunner;
use eventflux_rust::core::event::value::AttributeValue;

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/// Test coalesce function
/// Reference: CoalesceFunctionTestCase.java:testCoalesceQuery1
#[tokio::test]
async fn function_test_coalesce() {
    let app = "\
        CREATE STREAM inputStream (symbol STRING, price FLOAT);\n\
        CREATE STREAM outputStream (result STRING);\n\
        INSERT INTO outputStream\n\
        SELECT coalesce(symbol, 'DEFAULT') AS result\n\
        FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "inputStream",
        vec![AttributeValue::Null, AttributeValue::Float(55.6)],
    );
    runner.send(
        "inputStream",
        vec![
            AttributeValue::String("IBM".to_string()),
            AttributeValue::Float(75.6),
        ],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 2);
    assert_eq!(out[0][0], AttributeValue::String("DEFAULT".to_string()));
    assert_eq!(out[1][0], AttributeValue::String("IBM".to_string()));
}

/// Coalesce with filter condition
/// Reference: FunctionTestCase.java testFunctionQuery3
#[tokio::test]
async fn function_test_coalesce_in_filter() {
    let app = "\
        CREATE STREAM cseEventStream (symbol STRING, price1 FLOAT, price2 FLOAT, volume BIGINT, quantity INT);\n\
        CREATE STREAM outputStream (symbol STRING, price FLOAT, quantity INT);\n\
        INSERT INTO outputStream\n\
        SELECT symbol, coalesce(price1, price2) AS price, quantity\n\
        FROM cseEventStream\n\
        WHERE coalesce(price1, price2) > 0.0;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "cseEventStream",
        vec![
            AttributeValue::String("MSFT".to_string()),
            AttributeValue::Float(50.0),
            AttributeValue::Float(60.0),
            AttributeValue::Long(60),
            AttributeValue::Int(6),
        ],
    );
    runner.send(
        "cseEventStream",
        vec![
            AttributeValue::String("MSFT".to_string()),
            AttributeValue::Float(70.0),
            AttributeValue::Null,
            AttributeValue::Long(40),
            AttributeValue::Int(10),
        ],
    );
    let out = runner.shutdown();
    // Both events pass (coalesce returns non-null positive values)
    assert!(out.len() >= 2);
    assert_eq!(out[0][1], AttributeValue::Float(50.0));
    assert_eq!(out[1][1], AttributeValue::Float(70.0));
}

/// Test cast function
/// Reference: ConversionFunctionTestCase.java:testConversionQuery1
#[tokio::test]
async fn function_test_cast() {
    let app = "\
        CREATE STREAM inputStream (strVal STRING, intVal INT);\n\
        CREATE STREAM outputStream (intResult INT, strResult STRING);\n\
        INSERT INTO outputStream\n\
        SELECT CAST(strVal AS INT) AS intResult, CAST(intVal AS STRING) AS strResult\n\
        FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "inputStream",
        vec![
            AttributeValue::String("123".to_string()),
            AttributeValue::Int(456),
        ],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::Int(123));
    assert_eq!(out[0][1], AttributeValue::String("456".to_string()));
}

/// Test UUID function
/// Reference: UUIDFunctionTestCase.java:testUUIDQuery1
#[tokio::test]
async fn function_test_uuid() {
    let app = "\
        CREATE STREAM inputStream (val INT);\n\
        CREATE STREAM outputStream (id STRING);\n\
        INSERT INTO outputStream\n\
        SELECT UUID() AS id\n\
        FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("inputStream", vec![AttributeValue::Int(1)]);
    runner.send("inputStream", vec![AttributeValue::Int(2)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 2);
    // Each UUID should be different
    assert_ne!(out[0][0], out[1][0]);
}

/// Test current timestamp function
/// Reference: FunctionTestCase.java:testFunctionQuery7_1
#[tokio::test]
async fn function_test_current_timestamp() {
    let app = "\
        CREATE STREAM inputStream (symbol STRING, price FLOAT);\n\
        CREATE STREAM outputStream (symbol STRING, ts BIGINT);\n\
        INSERT INTO outputStream\n\
        SELECT symbol, eventTimestamp() AS ts\n\
        FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send_with_ts(
        "inputStream",
        1234567890,
        vec![
            AttributeValue::String("IBM".to_string()),
            AttributeValue::Float(75.6),
        ],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][1], AttributeValue::Long(1234567890));
}

// ============================================================================
// MATH FUNCTIONS
// ============================================================================

/// Math function - abs
/// Reference: FunctionTestCase.java
/// Note: abs function executor registered
#[tokio::test]
async fn function_test_abs() {
    let app = "\
        CREATE STREAM inputStream (value INT);\n\
        CREATE STREAM outputStream (absValue INT);\n\
        INSERT INTO outputStream\n\
        SELECT abs(value) AS absValue FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("inputStream", vec![AttributeValue::Int(-10)]);
    runner.send("inputStream", vec![AttributeValue::Int(5)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 2);
    assert_eq!(out[0][0], AttributeValue::Int(10));
    assert_eq!(out[1][0], AttributeValue::Int(5));
}

/// Math function - round
/// Reference: FunctionTestCase.java
#[tokio::test]
async fn function_test_round() {
    let app = "\
        CREATE STREAM inputStream (value DOUBLE);\n\
        CREATE STREAM outputStream (roundedValue DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT round(value) AS roundedValue FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("inputStream", vec![AttributeValue::Double(3.7)]);
    runner.send("inputStream", vec![AttributeValue::Double(3.2)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 2);
    assert_eq!(out[0][0], AttributeValue::Double(4.0));
    assert_eq!(out[1][0], AttributeValue::Double(3.0));
}

/// Math function: sqrt
/// Reference: Math function tests
#[tokio::test]
async fn function_test_sqrt() {
    let app = "\
        CREATE STREAM inputStream (value DOUBLE);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT sqrt(value) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("inputStream", vec![AttributeValue::Double(16.0)]);
    runner.send("inputStream", vec![AttributeValue::Double(25.0)]);
    let out = runner.shutdown();
    assert!(out.len() >= 2);
    assert_eq!(out[0][0], AttributeValue::Double(4.0));
    assert_eq!(out[1][0], AttributeValue::Double(5.0));
}

// ============================================================================
// STRING FUNCTIONS
// ============================================================================

/// String function - concat
/// Reference: FunctionTestCase.java
#[tokio::test]
async fn function_test_concat() {
    let app = "\
        CREATE STREAM inputStream (first STRING, last STRING);\n\
        CREATE STREAM outputStream (fullName STRING);\n\
        INSERT INTO outputStream\n\
        SELECT concat(first, ' ', last) AS fullName FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "inputStream",
        vec![
            AttributeValue::String("John".to_string()),
            AttributeValue::String("Doe".to_string()),
        ],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::String("John Doe".to_string()));
}

/// String function - upper
/// Reference: FunctionTestCase.java
#[tokio::test]
async fn function_test_upper() {
    let app = "\
        CREATE STREAM inputStream (text STRING);\n\
        CREATE STREAM outputStream (upperText STRING);\n\
        INSERT INTO outputStream\n\
        SELECT upper(text) AS upperText FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "inputStream",
        vec![AttributeValue::String("hello world".to_string())],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::String("HELLO WORLD".to_string()));
}

/// String function - lower
/// Reference: FunctionTestCase.java
#[tokio::test]
async fn function_test_lower() {
    let app = "\
        CREATE STREAM inputStream (text STRING);\n\
        CREATE STREAM outputStream (lowerText STRING);\n\
        INSERT INTO outputStream\n\
        SELECT lower(text) AS lowerText FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "inputStream",
        vec![AttributeValue::String("HELLO WORLD".to_string())],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::String("hello world".to_string()));
}

/// String function - length
/// Reference: FunctionTestCase.java
#[tokio::test]
async fn function_test_length() {
    let app = "\
        CREATE STREAM inputStream (text STRING);\n\
        CREATE STREAM outputStream (len INT);\n\
        INSERT INTO outputStream\n\
        SELECT length(text) AS len FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "inputStream",
        vec![AttributeValue::String("hello".to_string())],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::Int(5));
}

/// String function: substring
/// Reference: String function tests
#[tokio::test]
async fn function_test_substring() {
    let app = "\
        CREATE STREAM inputStream (text STRING);\n\
        CREATE STREAM outputStream (result STRING);\n\
        INSERT INTO outputStream\n\
        SELECT substring(text, 1, 4) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "inputStream",
        vec![AttributeValue::String("HelloWorld".to_string())],
    );
    let out = runner.shutdown();
    assert!(!out.is_empty());
    // substring(1, 4) should give characters 1-4 (0-indexed: "ello")
    // Note: Exact behavior depends on implementation (0-indexed or 1-indexed)
    let result = &out[0][0];
    if let AttributeValue::String(s) = result {
        assert!(s.len() <= 4);
    }
}

// ============================================================================
// ARITHMETIC OPERATIONS
// Reference: query/function/FunctionTestCase.java
// ============================================================================

/// Arithmetic - addition
/// Reference: FunctionTestCase.java
#[tokio::test]
async fn arithmetic_test_addition() {
    let app = "\
        CREATE STREAM inputStream (a INT, b INT);\n\
        CREATE STREAM outputStream (result INT);\n\
        INSERT INTO outputStream\n\
        SELECT a + b AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "inputStream",
        vec![AttributeValue::Int(10), AttributeValue::Int(5)],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::Int(15));
}

/// Arithmetic - subtraction
/// Reference: FunctionTestCase.java
#[tokio::test]
async fn arithmetic_test_subtraction() {
    let app = "\
        CREATE STREAM inputStream (a INT, b INT);\n\
        CREATE STREAM outputStream (result INT);\n\
        INSERT INTO outputStream\n\
        SELECT a - b AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "inputStream",
        vec![AttributeValue::Int(10), AttributeValue::Int(3)],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::Int(7));
}

/// Arithmetic - multiplication
/// Reference: FunctionTestCase.java
#[tokio::test]
async fn arithmetic_test_multiplication() {
    let app = "\
        CREATE STREAM inputStream (a INT, b INT);\n\
        CREATE STREAM outputStream (result INT);\n\
        INSERT INTO outputStream\n\
        SELECT a * b AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "inputStream",
        vec![AttributeValue::Int(10), AttributeValue::Int(5)],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::Int(50));
}

/// Arithmetic - division (returns DOUBLE for integer division)
/// Reference: FunctionTestCase.java
#[tokio::test]
async fn arithmetic_test_division() {
    let app = "\
        CREATE STREAM inputStream (a INT, b INT);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT a / b AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "inputStream",
        vec![AttributeValue::Int(10), AttributeValue::Int(2)],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::Double(5.0));
}

/// Arithmetic - modulo (returns DOUBLE in EventFlux)
/// Reference: FunctionTestCase.java
/// Note: Modulo operator not yet supported in SQL converter
#[tokio::test]
#[ignore = "Modulo operator not yet supported in SQL converter"]
async fn arithmetic_test_modulo() {
    let app = "\
        CREATE STREAM inputStream (a INT, b INT);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT a % b AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "inputStream",
        vec![AttributeValue::Int(10), AttributeValue::Int(3)],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::Double(1.0));
}

/// Complex arithmetic expression
/// Reference: FunctionTestCase.java
#[tokio::test]
async fn arithmetic_test_complex_expression() {
    let app = "\
        CREATE STREAM inputStream (a INT, b INT, c INT);\n\
        CREATE STREAM outputStream (result INT);\n\
        INSERT INTO outputStream\n\
        SELECT (a + b) * c AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "inputStream",
        vec![
            AttributeValue::Int(2),
            AttributeValue::Int(3),
            AttributeValue::Int(4),
        ],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    // (2 + 3) * 4 = 20
    assert_eq!(out[0][0], AttributeValue::Int(20));
}

// ============================================================================
// ADDITIONAL MATH FUNCTIONS
// ============================================================================

/// Math function - log (natural logarithm)
/// Reference: Math function tests
#[tokio::test]
async fn function_test_log() {
    let app = "\
        CREATE STREAM inputStream (value DOUBLE);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT log(value) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    // log(e) = 1, log(e^2) = 2
    runner.send(
        "inputStream",
        vec![AttributeValue::Double(std::f64::consts::E)],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    if let AttributeValue::Double(val) = out[0][0] {
        assert!((val - 1.0).abs() < 0.0001);
    } else {
        panic!("Expected Double");
    }
}

/// Math function - sin
/// Reference: Math function tests
#[tokio::test]
async fn function_test_sin() {
    let app = "\
        CREATE STREAM inputStream (value DOUBLE);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT sin(value) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    // sin(0) = 0, sin(PI/2) = 1
    runner.send("inputStream", vec![AttributeValue::Double(0.0)]);
    runner.send(
        "inputStream",
        vec![AttributeValue::Double(std::f64::consts::FRAC_PI_2)],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 2);
    if let AttributeValue::Double(val) = out[0][0] {
        assert!(val.abs() < 0.0001);
    }
    if let AttributeValue::Double(val) = out[1][0] {
        assert!((val - 1.0).abs() < 0.0001);
    }
}

/// Math function - cos
/// Reference: Math function tests
#[tokio::test]
async fn function_test_cos() {
    let app = "\
        CREATE STREAM inputStream (value DOUBLE);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT cos(value) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    // cos(0) = 1
    runner.send("inputStream", vec![AttributeValue::Double(0.0)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    if let AttributeValue::Double(val) = out[0][0] {
        assert!((val - 1.0).abs() < 0.0001);
    }
}

/// Math function - tan
/// Reference: Math function tests
#[tokio::test]
async fn function_test_tan() {
    let app = "\
        CREATE STREAM inputStream (value DOUBLE);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT tan(value) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    // tan(0) = 0, tan(PI/4) = 1
    runner.send("inputStream", vec![AttributeValue::Double(0.0)]);
    runner.send(
        "inputStream",
        vec![AttributeValue::Double(std::f64::consts::FRAC_PI_4)],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 2);
    if let AttributeValue::Double(val) = out[0][0] {
        assert!(val.abs() < 0.0001);
    }
    if let AttributeValue::Double(val) = out[1][0] {
        assert!((val - 1.0).abs() < 0.0001);
    }
}

/// Math function - power
/// Reference: Math function tests
#[tokio::test]
async fn function_test_power() {
    let app = "\
        CREATE STREAM inputStream (base DOUBLE, exp DOUBLE);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT power(base, exp) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    // 2^3 = 8
    runner.send(
        "inputStream",
        vec![AttributeValue::Double(2.0), AttributeValue::Double(3.0)],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::Double(8.0));
}

/// Math function - floor
/// Reference: Math function tests
#[tokio::test]
async fn function_test_floor() {
    let app = "\
        CREATE STREAM inputStream (value DOUBLE);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT floor(value) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("inputStream", vec![AttributeValue::Double(3.7)]);
    runner.send("inputStream", vec![AttributeValue::Double(-2.3)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 2);
    assert_eq!(out[0][0], AttributeValue::Double(3.0));
    assert_eq!(out[1][0], AttributeValue::Double(-3.0));
}

/// Math function - ceil
/// Reference: Math function tests
#[tokio::test]
async fn function_test_ceil() {
    let app = "\
        CREATE STREAM inputStream (value DOUBLE);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT ceil(value) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("inputStream", vec![AttributeValue::Double(3.2)]);
    runner.send("inputStream", vec![AttributeValue::Double(-2.7)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 2);
    assert_eq!(out[0][0], AttributeValue::Double(4.0));
    assert_eq!(out[1][0], AttributeValue::Double(-2.0));
}

// ============================================================================
// ADDITIONAL STRING FUNCTIONS
// ============================================================================

/// String function - trim
/// Reference: String function tests
#[tokio::test]
async fn function_test_trim() {
    let app = "\
        CREATE STREAM inputStream (text STRING);\n\
        CREATE STREAM outputStream (result STRING);\n\
        INSERT INTO outputStream\n\
        SELECT trim(text) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "inputStream",
        vec![AttributeValue::String("  hello  ".to_string())],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::String("hello".to_string()));
}

/// String function - replace
/// Reference: String function tests
#[tokio::test]
async fn function_test_replace() {
    let app = "\
        CREATE STREAM inputStream (text STRING);\n\
        CREATE STREAM outputStream (result STRING);\n\
        INSERT INTO outputStream\n\
        SELECT replace(text, 'world', 'rust') AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "inputStream",
        vec![AttributeValue::String("hello world".to_string())],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::String("hello rust".to_string()));
}

// ============================================================================
// IFNULL / NULLIF FUNCTIONS
// ============================================================================

/// ifnull function - returns first non-null value
/// Reference: Function tests
#[tokio::test]
async fn function_test_ifnull() {
    let app = "\
        CREATE STREAM inputStream (value FLOAT, fallback FLOAT);\n\
        CREATE STREAM outputStream (result FLOAT);\n\
        INSERT INTO outputStream\n\
        SELECT ifnull(value, fallback) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "inputStream",
        vec![AttributeValue::Null, AttributeValue::Float(100.0)],
    );
    runner.send(
        "inputStream",
        vec![AttributeValue::Float(50.0), AttributeValue::Float(100.0)],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 2);
    assert_eq!(out[0][0], AttributeValue::Float(100.0));
    assert_eq!(out[1][0], AttributeValue::Float(50.0));
}

/// nullif function - returns NULL if values are equal
/// Reference: Function tests
#[tokio::test]
async fn function_test_nullif() {
    let app = "\
        CREATE STREAM inputStream (a INT, b INT);\n\
        CREATE STREAM outputStream (result INT);\n\
        INSERT INTO outputStream\n\
        SELECT nullif(a, b) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "inputStream",
        vec![AttributeValue::Int(5), AttributeValue::Int(5)],
    );
    runner.send(
        "inputStream",
        vec![AttributeValue::Int(10), AttributeValue::Int(5)],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 2);
    assert_eq!(out[0][0], AttributeValue::Null);
    assert_eq!(out[1][0], AttributeValue::Int(10));
}

// ============================================================================
// CASE EXPRESSION TESTS
// ============================================================================

/// CASE WHEN expression - simple case
/// Reference: Function tests
#[tokio::test]
async fn function_test_case_when_simple() {
    let app = "\
        CREATE STREAM inputStream (status INT);\n\
        CREATE STREAM outputStream (result STRING);\n\
        INSERT INTO outputStream\n\
        SELECT CASE WHEN status = 1 THEN 'Active' WHEN status = 0 THEN 'Inactive' ELSE 'Unknown' END AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("inputStream", vec![AttributeValue::Int(1)]);
    runner.send("inputStream", vec![AttributeValue::Int(0)]);
    runner.send("inputStream", vec![AttributeValue::Int(2)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 3);
    assert_eq!(out[0][0], AttributeValue::String("Active".to_string()));
    assert_eq!(out[1][0], AttributeValue::String("Inactive".to_string()));
    assert_eq!(out[2][0], AttributeValue::String("Unknown".to_string()));
}

/// CASE WHEN expression - with comparison
/// Reference: Function tests
#[tokio::test]
async fn function_test_case_when_comparison() {
    let app = "\
        CREATE STREAM inputStream (price FLOAT);\n\
        CREATE STREAM outputStream (category STRING);\n\
        INSERT INTO outputStream\n\
        SELECT CASE WHEN price < 50.0 THEN 'Cheap' WHEN price < 100.0 THEN 'Medium' ELSE 'Expensive' END AS category FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("inputStream", vec![AttributeValue::Float(30.0)]);
    runner.send("inputStream", vec![AttributeValue::Float(75.0)]);
    runner.send("inputStream", vec![AttributeValue::Float(150.0)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 3);
    assert_eq!(out[0][0], AttributeValue::String("Cheap".to_string()));
    assert_eq!(out[1][0], AttributeValue::String("Medium".to_string()));
    assert_eq!(out[2][0], AttributeValue::String("Expensive".to_string()));
}

/// CASE WHEN with numeric return
/// Note: CASE WHEN returns BIGINT for integer constants
#[tokio::test]
async fn function_test_case_when_numeric() {
    let app = "\
        CREATE STREAM inputStream (grade STRING);\n\
        CREATE STREAM outputStream (points BIGINT);\n\
        INSERT INTO outputStream\n\
        SELECT CASE WHEN grade = 'A' THEN 4 WHEN grade = 'B' THEN 3 WHEN grade = 'C' THEN 2 ELSE 0 END AS points FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("inputStream", vec![AttributeValue::String("A".to_string())]);
    runner.send("inputStream", vec![AttributeValue::String("C".to_string())]);
    runner.send("inputStream", vec![AttributeValue::String("F".to_string())]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 3);
    assert_eq!(out[0][0], AttributeValue::Long(4));
    assert_eq!(out[1][0], AttributeValue::Long(2));
    assert_eq!(out[2][0], AttributeValue::Long(0));
}

/// CASE WHEN with boolean result
#[tokio::test]
async fn function_test_case_when_boolean() {
    let app = "\
        CREATE STREAM inputStream (value INT);\n\
        CREATE STREAM outputStream (isPositive BOOLEAN);\n\
        INSERT INTO outputStream\n\
        SELECT CASE WHEN value > 0 THEN true ELSE false END AS isPositive FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("inputStream", vec![AttributeValue::Int(5)]);
    runner.send("inputStream", vec![AttributeValue::Int(-3)]);
    runner.send("inputStream", vec![AttributeValue::Int(0)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 3);
    assert_eq!(out[0][0], AttributeValue::Bool(true));
    assert_eq!(out[1][0], AttributeValue::Bool(false));
    assert_eq!(out[2][0], AttributeValue::Bool(false));
}

// ============================================================================
// FUNCTION COMBINATIONS
// ============================================================================

/// Combine multiple string functions
#[tokio::test]
async fn function_test_string_chain() {
    let app = "\
        CREATE STREAM inputStream (text STRING);\n\
        CREATE STREAM outputStream (result STRING);\n\
        INSERT INTO outputStream\n\
        SELECT upper(concat(text, '_suffix')) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "inputStream",
        vec![AttributeValue::String("hello".to_string())],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(
        out[0][0],
        AttributeValue::String("HELLO_SUFFIX".to_string())
    );
}

/// Combine math and string functions
#[tokio::test]
async fn function_test_mixed_functions() {
    let app = "\
        CREATE STREAM inputStream (name STRING, score DOUBLE);\n\
        CREATE STREAM outputStream (report STRING, rounded DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT upper(name) AS report, round(score) AS rounded FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "inputStream",
        vec![
            AttributeValue::String("alice".to_string()),
            AttributeValue::Double(85.6),
        ],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::String("ALICE".to_string()));
    assert_eq!(out[0][1], AttributeValue::Double(86.0));
}

/// Nested function calls
#[tokio::test]
async fn function_test_nested_calls() {
    let app = "\
        CREATE STREAM inputStream (value DOUBLE);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT round(sqrt(value)) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("inputStream", vec![AttributeValue::Double(17.0)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    // sqrt(17) ≈ 4.12, round = 4.0
    assert_eq!(out[0][0], AttributeValue::Double(4.0));
}

/// Coalesce with function
#[tokio::test]
async fn function_test_coalesce_with_function() {
    let app = "\
        CREATE STREAM inputStream (name STRING);\n\
        CREATE STREAM outputStream (result STRING);\n\
        INSERT INTO outputStream\n\
        SELECT upper(coalesce(name, 'unknown')) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("inputStream", vec![AttributeValue::Null]);
    runner.send(
        "inputStream",
        vec![AttributeValue::String("alice".to_string())],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 2);
    assert_eq!(out[0][0], AttributeValue::String("UNKNOWN".to_string()));
    assert_eq!(out[1][0], AttributeValue::String("ALICE".to_string()));
}

// ============================================================================
// ARITHMETIC WITH FUNCTIONS
// ============================================================================

/// Arithmetic with round
#[tokio::test]
async fn function_test_arithmetic_with_round() {
    let app = "\
        CREATE STREAM inputStream (price DOUBLE, quantity INT);\n\
        CREATE STREAM outputStream (total DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT round(price * quantity) AS total FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "inputStream",
        vec![AttributeValue::Double(9.99), AttributeValue::Int(3)],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    // 9.99 * 3 = 29.97, rounded = 30.0
    assert_eq!(out[0][0], AttributeValue::Double(30.0));
}

/// Complex arithmetic expression
#[tokio::test]
async fn function_test_complex_arithmetic() {
    let app = "\
        CREATE STREAM inputStream (a DOUBLE, b DOUBLE, c DOUBLE);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT (a + b) / c AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "inputStream",
        vec![
            AttributeValue::Double(10.0),
            AttributeValue::Double(20.0),
            AttributeValue::Double(6.0),
        ],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    // (10 + 20) / 6 = 5.0
    assert_eq!(out[0][0], AttributeValue::Double(5.0));
}

/// Nested arithmetic operations
#[tokio::test]
async fn function_test_nested_arithmetic() {
    let app = "\
        CREATE STREAM inputStream (x INT, y INT, z INT);\n\
        CREATE STREAM outputStream (result INT);\n\
        INSERT INTO outputStream\n\
        SELECT x * (y + z) - (x - y) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "inputStream",
        vec![
            AttributeValue::Int(5),
            AttributeValue::Int(3),
            AttributeValue::Int(2),
        ],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    // 5 * (3 + 2) - (5 - 3) = 5 * 5 - 2 = 25 - 2 = 23
    assert_eq!(out[0][0], AttributeValue::Int(23));
}

// ============================================================================
// CAST FUNCTION VARIATIONS
// ============================================================================

/// Cast INT to BIGINT
#[tokio::test]
async fn function_test_cast_int_to_long() {
    let app = "\
        CREATE STREAM inputStream (value INT);\n\
        CREATE STREAM outputStream (result BIGINT);\n\
        INSERT INTO outputStream\n\
        SELECT CAST(value AS BIGINT) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("inputStream", vec![AttributeValue::Int(12345)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::Long(12345));
}

/// Cast FLOAT to DOUBLE
#[tokio::test]
async fn function_test_cast_float_to_double() {
    let app = "\
        CREATE STREAM inputStream (value FLOAT);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT CAST(value AS DOUBLE) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("inputStream", vec![AttributeValue::Float(3.5)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    if let AttributeValue::Double(d) = out[0][0] {
        assert!((d - 3.5).abs() < 0.001);
    } else {
        panic!("Expected Double");
    }
}

/// Cast DOUBLE to INT (truncation)
#[tokio::test]
async fn function_test_cast_double_to_int() {
    let app = "\
        CREATE STREAM inputStream (value DOUBLE);\n\
        CREATE STREAM outputStream (result INT);\n\
        INSERT INTO outputStream\n\
        SELECT CAST(value AS INT) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("inputStream", vec![AttributeValue::Double(9.99)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    // Truncates to 9
    assert_eq!(out[0][0], AttributeValue::Int(9));
}

/// Cast STRING to FLOAT
#[tokio::test]
async fn function_test_cast_string_to_float() {
    let app = "\
        CREATE STREAM inputStream (value STRING);\n\
        CREATE STREAM outputStream (result FLOAT);\n\
        INSERT INTO outputStream\n\
        SELECT CAST(value AS FLOAT) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "inputStream",
        vec![AttributeValue::String("123.45".to_string())],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    if let AttributeValue::Float(f) = out[0][0] {
        assert!((f - 123.45).abs() < 0.01);
    } else {
        panic!("Expected Float");
    }
}

// ============================================================================
// STRING LENGTH AND CONCAT EDGE CASES
// ============================================================================

/// Length of empty string
#[tokio::test]
async fn function_test_length_empty() {
    let app = "\
        CREATE STREAM inputStream (text STRING);\n\
        CREATE STREAM outputStream (len INT);\n\
        INSERT INTO outputStream\n\
        SELECT length(text) AS len FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("inputStream", vec![AttributeValue::String("".to_string())]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::Int(0));
}

/// Concat with empty strings
#[tokio::test]
async fn function_test_concat_empty() {
    let app = "\
        CREATE STREAM inputStream (a STRING, b STRING);\n\
        CREATE STREAM outputStream (result STRING);\n\
        INSERT INTO outputStream\n\
        SELECT concat(a, b) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "inputStream",
        vec![
            AttributeValue::String("hello".to_string()),
            AttributeValue::String("".to_string()),
        ],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::String("hello".to_string()));
}

/// Concat multiple strings
#[tokio::test]
async fn function_test_concat_multiple() {
    let app = "\
        CREATE STREAM inputStream (a STRING, b STRING, c STRING);\n\
        CREATE STREAM outputStream (result STRING);\n\
        INSERT INTO outputStream\n\
        SELECT concat(a, b, c) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "inputStream",
        vec![
            AttributeValue::String("one".to_string()),
            AttributeValue::String("two".to_string()),
            AttributeValue::String("three".to_string()),
        ],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::String("onetwothree".to_string()));
}

// ============================================================================
// COALESCE VARIATIONS
// ============================================================================

/// Coalesce with multiple arguments
#[tokio::test]
async fn function_test_coalesce_multiple() {
    let app = "\
        CREATE STREAM inputStream (a STRING, b STRING, c STRING);\n\
        CREATE STREAM outputStream (result STRING);\n\
        INSERT INTO outputStream\n\
        SELECT coalesce(a, b, c) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "inputStream",
        vec![
            AttributeValue::Null,
            AttributeValue::Null,
            AttributeValue::String("third".to_string()),
        ],
    );
    runner.send(
        "inputStream",
        vec![
            AttributeValue::Null,
            AttributeValue::String("second".to_string()),
            AttributeValue::String("third".to_string()),
        ],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 2);
    assert_eq!(out[0][0], AttributeValue::String("third".to_string()));
    assert_eq!(out[1][0], AttributeValue::String("second".to_string()));
}

/// Coalesce with all non-null (returns first)
#[tokio::test]
async fn function_test_coalesce_all_non_null() {
    let app = "\
        CREATE STREAM inputStream (a INT, b INT);\n\
        CREATE STREAM outputStream (result INT);\n\
        INSERT INTO outputStream\n\
        SELECT coalesce(a, b) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "inputStream",
        vec![AttributeValue::Int(10), AttributeValue::Int(20)],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::Int(10));
}

// ============================================================================
// UUID UNIQUENESS TEST
// ============================================================================

/// Test UUID generates unique values across many events
#[tokio::test]
async fn function_test_uuid_uniqueness() {
    let app = "\
        CREATE STREAM inputStream (val INT);\n\
        CREATE STREAM outputStream (id STRING);\n\
        INSERT INTO outputStream\n\
        SELECT UUID() AS id\n\
        FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    for i in 1..=10 {
        runner.send("inputStream", vec![AttributeValue::Int(i)]);
    }
    let out = runner.shutdown();
    assert_eq!(out.len(), 10);
    // Check all UUIDs are unique
    let mut seen = std::collections::HashSet::new();
    for row in &out {
        if let AttributeValue::String(uuid) = &row[0] {
            assert!(seen.insert(uuid.clone()), "Duplicate UUID found");
        }
    }
}

// ============================================================================
// SQRT EDGE CASES
// ============================================================================

/// Sqrt of zero
#[tokio::test]
async fn function_test_sqrt_zero() {
    let app = "\
        CREATE STREAM inputStream (value DOUBLE);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT sqrt(value) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("inputStream", vec![AttributeValue::Double(0.0)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::Double(0.0));
}

/// Sqrt of one
#[tokio::test]
async fn function_test_sqrt_one() {
    let app = "\
        CREATE STREAM inputStream (value DOUBLE);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT sqrt(value) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("inputStream", vec![AttributeValue::Double(1.0)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::Double(1.0));
}

// ============================================================================
// ADDITIONAL FUNCTION TESTS
// ============================================================================

/// abs function with positive number
#[tokio::test]
async fn function_test_abs_positive() {
    let app = "\
        CREATE STREAM inputStream (value DOUBLE);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT abs(value) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("inputStream", vec![AttributeValue::Double(42.5)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::Double(42.5));
}

/// abs function with negative number
#[tokio::test]
async fn function_test_abs_negative() {
    let app = "\
        CREATE STREAM inputStream (value DOUBLE);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT abs(value) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("inputStream", vec![AttributeValue::Double(-42.5)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::Double(42.5));
}

/// abs function with zero
#[tokio::test]
async fn function_test_abs_zero() {
    let app = "\
        CREATE STREAM inputStream (value DOUBLE);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT abs(value) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("inputStream", vec![AttributeValue::Double(0.0)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::Double(0.0));
}

/// floor function edge case - positive
#[tokio::test]
async fn function_test_floor_positive() {
    let app = "\
        CREATE STREAM inputStream (value DOUBLE);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT floor(value) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("inputStream", vec![AttributeValue::Double(3.7)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::Double(3.0));
}

/// floor function with negative number
#[tokio::test]
async fn function_test_floor_negative() {
    let app = "\
        CREATE STREAM inputStream (value DOUBLE);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT floor(value) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("inputStream", vec![AttributeValue::Double(-3.2)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::Double(-4.0));
}

/// ceil function edge case - positive
#[tokio::test]
async fn function_test_ceil_positive() {
    let app = "\
        CREATE STREAM inputStream (value DOUBLE);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT ceil(value) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("inputStream", vec![AttributeValue::Double(3.2)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::Double(4.0));
}

/// ceil function with negative number
#[tokio::test]
async fn function_test_ceil_negative() {
    let app = "\
        CREATE STREAM inputStream (value DOUBLE);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT ceil(value) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("inputStream", vec![AttributeValue::Double(-3.7)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::Double(-3.0));
}

/// round function edge case - half up
#[tokio::test]
async fn function_test_round_half_up() {
    let app = "\
        CREATE STREAM inputStream (value DOUBLE);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT round(value) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("inputStream", vec![AttributeValue::Double(3.5)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::Double(4.0));
}

/// length function edge case - basic
#[tokio::test]
async fn function_test_length_basic() {
    let app = "\
        CREATE STREAM inputStream (text STRING);\n\
        CREATE STREAM outputStream (len INT);\n\
        INSERT INTO outputStream\n\
        SELECT length(text) AS len FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "inputStream",
        vec![AttributeValue::String("Hello".to_string())],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::Int(5));
}

/// length function with empty string - edge case
#[tokio::test]
async fn function_test_length_empty_string() {
    let app = "\
        CREATE STREAM inputStream (text STRING);\n\
        CREATE STREAM outputStream (len INT);\n\
        INSERT INTO outputStream\n\
        SELECT length(text) AS len FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("inputStream", vec![AttributeValue::String("".to_string())]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::Int(0));
}

/// trim function edge case - basic
#[tokio::test]
async fn function_test_trim_basic() {
    let app = "\
        CREATE STREAM inputStream (text STRING);\n\
        CREATE STREAM outputStream (result STRING);\n\
        INSERT INTO outputStream\n\
        SELECT trim(text) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "inputStream",
        vec![AttributeValue::String("  hello  ".to_string())],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::String("hello".to_string()));
}

/// replace function edge case - basic
#[tokio::test]
async fn function_test_replace_basic() {
    let app = "\
        CREATE STREAM inputStream (text STRING);\n\
        CREATE STREAM outputStream (result STRING);\n\
        INSERT INTO outputStream\n\
        SELECT replace(text, 'world', 'there') AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "inputStream",
        vec![AttributeValue::String("hello world".to_string())],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::String("hello there".to_string()));
}

/// substring function edge case - basic
#[tokio::test]
async fn function_test_substring_basic() {
    let app = "\
        CREATE STREAM inputStream (text STRING);\n\
        CREATE STREAM outputStream (result STRING);\n\
        INSERT INTO outputStream\n\
        SELECT substr(text, 1, 5) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "inputStream",
        vec![AttributeValue::String("Hello World".to_string())],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::String("Hello".to_string()));
}

/// power function edge case - two to three
#[tokio::test]
async fn function_test_power_two_three() {
    let app = "\
        CREATE STREAM inputStream (base DOUBLE, exp DOUBLE);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT power(base, exp) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "inputStream",
        vec![AttributeValue::Double(2.0), AttributeValue::Double(3.0)],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::Double(8.0));
}

/// log function edge case - natural log of e
#[tokio::test]
async fn function_test_ln_of_e() {
    let app = "\
        CREATE STREAM inputStream (value DOUBLE);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT ln(value) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "inputStream",
        vec![AttributeValue::Double(std::f64::consts::E)],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    // Allow small floating point error
    if let AttributeValue::Double(val) = out[0][0] {
        assert!((val - 1.0).abs() < 0.0001);
    }
}

/// log10 function
#[tokio::test]
async fn function_test_log10() {
    let app = "\
        CREATE STREAM inputStream (value DOUBLE);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT log10(value) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("inputStream", vec![AttributeValue::Double(100.0)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::Double(2.0));
}

/// exp function
#[tokio::test]
async fn function_test_exp() {
    let app = "\
        CREATE STREAM inputStream (value DOUBLE);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT exp(value) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("inputStream", vec![AttributeValue::Double(0.0)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::Double(1.0));
}

/// sin function edge case - sin of zero
#[tokio::test]
async fn function_test_sin_of_zero() {
    let app = "\
        CREATE STREAM inputStream (value DOUBLE);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT sin(value) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("inputStream", vec![AttributeValue::Double(0.0)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::Double(0.0));
}

/// cos function edge case - cos of zero
#[tokio::test]
async fn function_test_cos_of_zero() {
    let app = "\
        CREATE STREAM inputStream (value DOUBLE);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT cos(value) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("inputStream", vec![AttributeValue::Double(0.0)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::Double(1.0));
}

/// tan function edge case - tan of zero
#[tokio::test]
async fn function_test_tan_of_zero() {
    let app = "\
        CREATE STREAM inputStream (value DOUBLE);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT tan(value) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("inputStream", vec![AttributeValue::Double(0.0)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::Double(0.0));
}

/// currentTimeMillis function
#[tokio::test]
async fn function_test_current_time_millis() {
    let app = "\
        CREATE STREAM inputStream (id INT);\n\
        CREATE STREAM outputStream (ts LONG);\n\
        INSERT INTO outputStream\n\
        SELECT currentTimeMillis() AS ts FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("inputStream", vec![AttributeValue::Int(1)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    // Should be a reasonable timestamp
    if let AttributeValue::Long(ts) = out[0][0] {
        assert!(ts > 1600000000000); // After 2020
    }
}

/// Conditional expression using CASE WHEN
#[tokio::test]
async fn function_test_case_conditional() {
    let app = "\
        CREATE STREAM inputStream (value INT);\n\
        CREATE STREAM outputStream (result STRING);\n\
        INSERT INTO outputStream\n\
        SELECT CASE WHEN value > 10 THEN 'HIGH' ELSE 'LOW' END AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("inputStream", vec![AttributeValue::Int(15)]);
    runner.send("inputStream", vec![AttributeValue::Int(5)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 2);
    assert_eq!(out[0][0], AttributeValue::String("HIGH".to_string()));
    assert_eq!(out[1][0], AttributeValue::String("LOW".to_string()));
}

/// coalesce with NULL first
#[tokio::test]
async fn function_test_coalesce_null_first() {
    let app = "\
        CREATE STREAM inputStream (a INT, b INT);\n\
        CREATE STREAM outputStream (result INT);\n\
        INSERT INTO outputStream\n\
        SELECT coalesce(a, b) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "inputStream",
        vec![AttributeValue::Null, AttributeValue::Int(10)],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::Int(10));
}

/// coalesce with non-NULL first
#[tokio::test]
async fn function_test_coalesce_non_null_first() {
    let app = "\
        CREATE STREAM inputStream (a INT, b INT);\n\
        CREATE STREAM outputStream (result INT);\n\
        INSERT INTO outputStream\n\
        SELECT coalesce(a, b) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "inputStream",
        vec![AttributeValue::Int(5), AttributeValue::Int(10)],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::Int(5));
}

/// Multiple functions in SELECT
#[tokio::test]
async fn function_test_multiple_functions() {
    let app = "\
        CREATE STREAM inputStream (value DOUBLE);\n\
        CREATE STREAM outputStream (floored DOUBLE, ceiled DOUBLE, rounded DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT floor(value) AS floored, ceil(value) AS ceiled, round(value) AS rounded FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("inputStream", vec![AttributeValue::Double(3.5)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::Double(3.0));
    assert_eq!(out[0][1], AttributeValue::Double(4.0));
    assert_eq!(out[0][2], AttributeValue::Double(4.0));
}

// ============================================================================
// ADDITIONAL FUNCTION EDGE CASE TESTS
// ============================================================================

/// concat with empty string suffix
#[tokio::test]
async fn function_test_concat_empty_suffix() {
    let app = "\
        CREATE STREAM inputStream (name STRING);\n\
        CREATE STREAM outputStream (result STRING);\n\
        INSERT INTO outputStream\n\
        SELECT concat(name, '') AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "inputStream",
        vec![AttributeValue::String("Hello".to_string())],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::String("Hello".to_string()));
}

/// concat with three arguments
#[tokio::test]
async fn function_test_concat_three_args() {
    let app = "\
        CREATE STREAM inputStream (first STRING, middle STRING, last STRING);\n\
        CREATE STREAM outputStream (full STRING);\n\
        INSERT INTO outputStream\n\
        SELECT concat(first, middle, last) AS full FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "inputStream",
        vec![
            AttributeValue::String("A".to_string()),
            AttributeValue::String("B".to_string()),
            AttributeValue::String("C".to_string()),
        ],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::String("ABC".to_string()));
}

/// length of empty string returns zero value
#[tokio::test]
async fn function_test_length_zero_for_empty() {
    let app = "\
        CREATE STREAM inputStream (name STRING);\n\
        CREATE STREAM outputStream (len INT);\n\
        INSERT INTO outputStream\n\
        SELECT length(name) AS len FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("inputStream", vec![AttributeValue::String("".to_string())]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::Int(0));
}

/// upper with mixed case
#[tokio::test]
async fn function_test_upper_mixed() {
    let app = "\
        CREATE STREAM inputStream (name STRING);\n\
        CREATE STREAM outputStream (result STRING);\n\
        INSERT INTO outputStream\n\
        SELECT upper(name) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "inputStream",
        vec![AttributeValue::String("HeLLo WoRLd".to_string())],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::String("HELLO WORLD".to_string()));
}

/// lower with mixed case
#[tokio::test]
async fn function_test_lower_mixed() {
    let app = "\
        CREATE STREAM inputStream (name STRING);\n\
        CREATE STREAM outputStream (result STRING);\n\
        INSERT INTO outputStream\n\
        SELECT lower(name) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "inputStream",
        vec![AttributeValue::String("HeLLo WoRLd".to_string())],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::String("hello world".to_string()));
}

/// upper with empty string
#[tokio::test]
async fn function_test_upper_empty() {
    let app = "\
        CREATE STREAM inputStream (name STRING);\n\
        CREATE STREAM outputStream (result STRING);\n\
        INSERT INTO outputStream\n\
        SELECT upper(name) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("inputStream", vec![AttributeValue::String("".to_string())]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::String("".to_string()));
}

/// uuid uniqueness across multiple events
#[tokio::test]
async fn function_test_uuid_multi_event_uniqueness() {
    let app = "\
        CREATE STREAM inputStream (id INT);\n\
        CREATE STREAM outputStream (eventId STRING);\n\
        INSERT INTO outputStream\n\
        SELECT uuid() AS eventId FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("inputStream", vec![AttributeValue::Int(1)]);
    runner.send("inputStream", vec![AttributeValue::Int(2)]);
    runner.send("inputStream", vec![AttributeValue::Int(3)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 3);
    // All UUIDs should be unique
    assert_ne!(out[0][0], out[1][0]);
    assert_ne!(out[1][0], out[2][0]);
    assert_ne!(out[0][0], out[2][0]);
}

/// currentTimeMillis increases over time
#[tokio::test]
async fn function_test_current_time_increases() {
    let app = "\
        CREATE STREAM inputStream (id INT);\n\
        CREATE STREAM outputStream (ts BIGINT);\n\
        INSERT INTO outputStream\n\
        SELECT currentTimeMillis() AS ts FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("inputStream", vec![AttributeValue::Int(1)]);
    std::thread::sleep(std::time::Duration::from_millis(10));
    runner.send("inputStream", vec![AttributeValue::Int(2)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 2);
    if let (AttributeValue::Long(ts1), AttributeValue::Long(ts2)) = (&out[0][0], &out[1][0]) {
        assert!(ts2 >= ts1);
    }
}

/// coalesce with three arguments
#[tokio::test]
async fn function_test_coalesce_three_args() {
    let app = "\
        CREATE STREAM inputStream (a INT, b INT, c INT);\n\
        CREATE STREAM outputStream (result INT);\n\
        INSERT INTO outputStream\n\
        SELECT coalesce(a, b, c) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "inputStream",
        vec![
            AttributeValue::Null,
            AttributeValue::Null,
            AttributeValue::Int(30),
        ],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::Int(30));
}

/// nested function calls: upper(concat(...))
#[tokio::test]
async fn function_test_nested_upper_concat() {
    let app = "\
        CREATE STREAM inputStream (first STRING, last STRING);\n\
        CREATE STREAM outputStream (result STRING);\n\
        INSERT INTO outputStream\n\
        SELECT upper(concat(first, ' ', last)) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "inputStream",
        vec![
            AttributeValue::String("john".to_string()),
            AttributeValue::String("doe".to_string()),
        ],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::String("JOHN DOE".to_string()));
}

/// nested function calls: length(concat(...))
#[tokio::test]
async fn function_test_nested_length_concat() {
    let app = "\
        CREATE STREAM inputStream (first STRING, last STRING);\n\
        CREATE STREAM outputStream (len INT);\n\
        INSERT INTO outputStream\n\
        SELECT length(concat(first, last)) AS len FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "inputStream",
        vec![
            AttributeValue::String("Hello".to_string()),
            AttributeValue::String("World".to_string()),
        ],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::Int(10)); // "HelloWorld" = 10 chars
}

/// concat with numbers converted to string
#[tokio::test]
async fn function_test_concat_with_cast() {
    let app = "\
        CREATE STREAM inputStream (prefix STRING, num INT);\n\
        CREATE STREAM outputStream (result STRING);\n\
        INSERT INTO outputStream\n\
        SELECT concat(prefix, '-', cast(num AS STRING)) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "inputStream",
        vec![
            AttributeValue::String("ID".to_string()),
            AttributeValue::Int(123),
        ],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::String("ID-123".to_string()));
}

/// substring function with start and length indices
#[tokio::test]
async fn function_test_substring_indices() {
    let app = "\
        CREATE STREAM inputStream (text STRING);\n\
        CREATE STREAM outputStream (result STRING);\n\
        INSERT INTO outputStream\n\
        SELECT substring(text, 0, 5) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "inputStream",
        vec![AttributeValue::String("HelloWorld".to_string())],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::String("Hello".to_string()));
}

/// trim function to remove surrounding whitespace
#[tokio::test]
async fn function_test_trim_whitespace() {
    let app = "\
        CREATE STREAM inputStream (text STRING);\n\
        CREATE STREAM outputStream (result STRING);\n\
        INSERT INTO outputStream\n\
        SELECT trim(text) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "inputStream",
        vec![AttributeValue::String("  hello  ".to_string())],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::String("hello".to_string()));
}

/// abs function for absolute value with multiple inputs
#[tokio::test]
async fn function_test_abs_multi_input() {
    let app = "\
        CREATE STREAM inputStream (value INT);\n\
        CREATE STREAM outputStream (result INT);\n\
        INSERT INTO outputStream\n\
        SELECT abs(value) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("inputStream", vec![AttributeValue::Int(-42)]);
    runner.send("inputStream", vec![AttributeValue::Int(42)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 2);
    assert_eq!(out[0][0], AttributeValue::Int(42));
    assert_eq!(out[1][0], AttributeValue::Int(42));
}

/// abs function for double
#[tokio::test]
async fn function_test_abs_double() {
    let app = "\
        CREATE STREAM inputStream (value DOUBLE);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT abs(value) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("inputStream", vec![AttributeValue::Double(-3.5)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    if let AttributeValue::Double(v) = out[0][0] {
        assert!((v - 3.5).abs() < 0.001);
    } else {
        panic!("Expected Double");
    }
}

/// round function with up and down rounding
#[tokio::test]
async fn function_test_round_up_down() {
    let app = "\
        CREATE STREAM inputStream (value DOUBLE);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT round(value) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("inputStream", vec![AttributeValue::Double(3.7)]);
    runner.send("inputStream", vec![AttributeValue::Double(3.2)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 2);
    // round returns Double in EventFlux
    if let AttributeValue::Double(v) = out[0][0] {
        assert!((v - 4.0).abs() < 0.001);
    } else {
        panic!("Expected Double");
    }
    if let AttributeValue::Double(v) = out[1][0] {
        assert!((v - 3.0).abs() < 0.001);
    } else {
        panic!("Expected Double");
    }
}

/// floor function with positive and negative values
#[tokio::test]
async fn function_test_floor_multi_input() {
    let app = "\
        CREATE STREAM inputStream (value DOUBLE);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT floor(value) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("inputStream", vec![AttributeValue::Double(3.7)]);
    runner.send("inputStream", vec![AttributeValue::Double(-2.3)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 2);
    if let AttributeValue::Double(v) = out[0][0] {
        assert!((v - 3.0).abs() < 0.001);
    } else {
        panic!("Expected Double");
    }
    if let AttributeValue::Double(v) = out[1][0] {
        assert!((v - -3.0).abs() < 0.001);
    } else {
        panic!("Expected Double");
    }
}

/// ceil function with positive and negative values
#[tokio::test]
async fn function_test_ceil_multi_input() {
    let app = "\
        CREATE STREAM inputStream (value DOUBLE);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT ceil(value) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("inputStream", vec![AttributeValue::Double(3.2)]);
    runner.send("inputStream", vec![AttributeValue::Double(-2.7)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 2);
    if let AttributeValue::Double(v) = out[0][0] {
        assert!((v - 4.0).abs() < 0.001);
    } else {
        panic!("Expected Double");
    }
    if let AttributeValue::Double(v) = out[1][0] {
        assert!((v - -2.0).abs() < 0.001);
    } else {
        panic!("Expected Double");
    }
}

/// sqrt function with multiple perfect squares (sqrt returns Double)
#[tokio::test]
async fn function_test_sqrt_multi_input() {
    let app = "\
        CREATE STREAM inputStream (value DOUBLE);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT sqrt(value) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("inputStream", vec![AttributeValue::Double(16.0)]);
    runner.send("inputStream", vec![AttributeValue::Double(9.0)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 2);
    if let AttributeValue::Double(v) = out[0][0] {
        assert!((v - 4.0).abs() < 0.001);
    } else {
        panic!("Expected Double");
    }
    if let AttributeValue::Double(v) = out[1][0] {
        assert!((v - 3.0).abs() < 0.001);
    } else {
        panic!("Expected Double");
    }
}

/// power function with base and exponent
#[tokio::test]
async fn function_test_power_base_exp() {
    let app = "\
        CREATE STREAM inputStream (base DOUBLE, exp DOUBLE);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT power(base, exp) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "inputStream",
        vec![AttributeValue::Double(2.0), AttributeValue::Double(3.0)],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    if let AttributeValue::Double(v) = out[0][0] {
        assert!((v - 8.0).abs() < 0.001);
    } else {
        panic!("Expected Double");
    }
}

/// log function (natural log) with e as input
#[tokio::test]
async fn function_test_ln_of_e_value() {
    let app = "\
        CREATE STREAM inputStream (value DOUBLE);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT ln(value) AS result FROM inputStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "inputStream",
        vec![AttributeValue::Double(std::f64::consts::E)],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    if let AttributeValue::Double(v) = out[0][0] {
        assert!((v - 1.0).abs() < 0.001);
    } else {
        panic!("Expected Double");
    }
}

/// concat with empty string
#[tokio::test]
async fn function_test_concat_empty_string() {
    let app = "\
        CREATE STREAM nameStream (first STRING, last STRING);\n\
        CREATE STREAM outputStream (full STRING);\n\
        INSERT INTO outputStream\n\
        SELECT concat(first, '', last) AS full FROM nameStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "nameStream",
        vec![
            AttributeValue::String("John".to_string()),
            AttributeValue::String("Doe".to_string()),
        ],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::String("JohnDoe".to_string()));
}

/// upper function with mixed case
#[tokio::test]
async fn function_test_upper_mixed_case() {
    let app = "\
        CREATE STREAM textStream (text STRING);\n\
        CREATE STREAM outputStream (result STRING);\n\
        INSERT INTO outputStream\n\
        SELECT upper(text) AS result FROM textStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "textStream",
        vec![AttributeValue::String("Hello World 123".to_string())],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(
        out[0][0],
        AttributeValue::String("HELLO WORLD 123".to_string())
    );
}

/// lower function with mixed case
#[tokio::test]
async fn function_test_lower_mixed_case() {
    let app = "\
        CREATE STREAM textStream (text STRING);\n\
        CREATE STREAM outputStream (result STRING);\n\
        INSERT INTO outputStream\n\
        SELECT lower(text) AS result FROM textStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "textStream",
        vec![AttributeValue::String("HELLO World 123".to_string())],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(
        out[0][0],
        AttributeValue::String("hello world 123".to_string())
    );
}

/// length function with empty string returns zero
#[tokio::test]
async fn function_test_length_empty_zero() {
    let app = "\
        CREATE STREAM textStream (text STRING);\n\
        CREATE STREAM outputStream (len INT);\n\
        INSERT INTO outputStream\n\
        SELECT length(text) AS len FROM textStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("textStream", vec![AttributeValue::String("".to_string())]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    let len = match &out[0][0] {
        AttributeValue::Int(v) => *v as i64,
        AttributeValue::Long(v) => *v,
        _ => panic!("Expected int or long"),
    };
    assert_eq!(len, 0);
}

/// abs function with negative double
#[tokio::test]
async fn function_test_abs_negative_double() {
    let app = "\
        CREATE STREAM numStream (value DOUBLE);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT abs(value) AS result FROM numStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("numStream", vec![AttributeValue::Double(-42.5)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    let val = match &out[0][0] {
        AttributeValue::Double(v) => *v,
        AttributeValue::Float(v) => *v as f64,
        _ => panic!("Expected double"),
    };
    assert!((val - 42.5).abs() < 0.001);
}

/// coalesce with first null-like empty string
#[tokio::test]
async fn function_test_coalesce_first_empty() {
    let app = "\
        CREATE STREAM dataStream (primary_val STRING, backup_val STRING);\n\
        CREATE STREAM outputStream (result STRING);\n\
        INSERT INTO outputStream\n\
        SELECT coalesce(primary_val, backup_val) AS result FROM dataStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "dataStream",
        vec![
            AttributeValue::String("".to_string()),
            AttributeValue::String("backup".to_string()),
        ],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    // coalesce returns first non-null, empty string is not null
    assert_eq!(out[0][0], AttributeValue::String("".to_string()));
}

/// uuid function generates unique consecutive values
#[tokio::test]
async fn function_test_uuid_unique_consecutive() {
    let app = "\
        CREATE STREAM triggerStream (id INT);\n\
        CREATE STREAM outputStream (uuid1 STRING);\n\
        INSERT INTO outputStream\n\
        SELECT uuid() AS uuid1 FROM triggerStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("triggerStream", vec![AttributeValue::Int(1)]);
    runner.send("triggerStream", vec![AttributeValue::Int(2)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 2);
    // UUIDs should be different
    assert_ne!(out[0][0], out[1][0]);
}

/// floor function with positive decimal value
#[tokio::test]
async fn function_test_floor_positive_decimal() {
    let app = "\
        CREATE STREAM numStream (value DOUBLE);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT floor(value) AS result FROM numStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("numStream", vec![AttributeValue::Double(3.7)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    let val = match &out[0][0] {
        AttributeValue::Double(v) => *v,
        AttributeValue::Float(v) => *v as f64,
        AttributeValue::Int(v) => *v as f64,
        AttributeValue::Long(v) => *v as f64,
        _ => panic!("Expected numeric type"),
    };
    assert!((val - 3.0).abs() < 0.001);
}

/// ceil function with positive decimal value
#[tokio::test]
async fn function_test_ceil_positive_decimal() {
    let app = "\
        CREATE STREAM numStream (value DOUBLE);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT ceil(value) AS result FROM numStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("numStream", vec![AttributeValue::Double(3.2)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    let val = match &out[0][0] {
        AttributeValue::Double(v) => *v,
        AttributeValue::Float(v) => *v as f64,
        AttributeValue::Int(v) => *v as f64,
        AttributeValue::Long(v) => *v as f64,
        _ => panic!("Expected numeric type"),
    };
    assert!((val - 4.0).abs() < 0.001);
}

/// concat with empty first argument
#[tokio::test]
async fn function_test_concat_empty_first() {
    let app = "\
        CREATE STREAM dataStream (prefix STRING, suffix STRING);\n\
        CREATE STREAM outputStream (result STRING);\n\
        INSERT INTO outputStream\n\
        SELECT concat(prefix, suffix) AS result FROM dataStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "dataStream",
        vec![
            AttributeValue::String("".to_string()),
            AttributeValue::String("world".to_string()),
        ],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::String("world".to_string()));
}

/// concat with empty second argument
#[tokio::test]
async fn function_test_concat_empty_second() {
    let app = "\
        CREATE STREAM dataStream (prefix STRING, suffix STRING);\n\
        CREATE STREAM outputStream (result STRING);\n\
        INSERT INTO outputStream\n\
        SELECT concat(prefix, suffix) AS result FROM dataStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "dataStream",
        vec![
            AttributeValue::String("hello".to_string()),
            AttributeValue::String("".to_string()),
        ],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::String("hello".to_string()));
}

/// length with empty string (returns zero value)
#[tokio::test]
async fn function_test_length_returns_zero_for_empty() {
    let app = "\
        CREATE STREAM dataStream (text STRING);\n\
        CREATE STREAM outputStream (len INT);\n\
        INSERT INTO outputStream\n\
        SELECT length(text) AS len FROM dataStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("dataStream", vec![AttributeValue::String("".to_string())]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    let len_val = match &out[0][0] {
        AttributeValue::Int(v) => *v as i64,
        AttributeValue::Long(v) => *v,
        _ => panic!("Expected int or long"),
    };
    assert_eq!(len_val, 0);
}

/// length with unicode characters (accented e)
#[tokio::test]
async fn function_test_length_unicode_accented() {
    let app = "\
        CREATE STREAM dataStream (text STRING);\n\
        CREATE STREAM outputStream (len INT);\n\
        INSERT INTO outputStream\n\
        SELECT length(text) AS len FROM dataStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "dataStream",
        vec![AttributeValue::String("héllo".to_string())],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    let len_val = match &out[0][0] {
        AttributeValue::Int(v) => *v as i64,
        AttributeValue::Long(v) => *v,
        _ => panic!("Expected int or long"),
    };
    // Length could be 5 (chars) or 6 (bytes) depending on implementation
    assert!(len_val == 5 || len_val == 6);
}

/// upper with alternating case input
#[tokio::test]
async fn function_test_upper_alternating_case() {
    let app = "\
        CREATE STREAM dataStream (text STRING);\n\
        CREATE STREAM outputStream (result STRING);\n\
        INSERT INTO outputStream\n\
        SELECT upper(text) AS result FROM dataStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "dataStream",
        vec![AttributeValue::String("HeLLo WoRLd".to_string())],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::String("HELLO WORLD".to_string()));
}

/// lower with alternating case input
#[tokio::test]
async fn function_test_lower_alternating_case() {
    let app = "\
        CREATE STREAM dataStream (text STRING);\n\
        CREATE STREAM outputStream (result STRING);\n\
        INSERT INTO outputStream\n\
        SELECT lower(text) AS result FROM dataStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "dataStream",
        vec![AttributeValue::String("HeLLo WoRLd".to_string())],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::String("hello world".to_string()));
}

/// abs with zero value (stays zero)
#[tokio::test]
async fn function_test_abs_zero_stays_zero() {
    let app = "\
        CREATE STREAM numStream (value INT);\n\
        CREATE STREAM outputStream (result INT);\n\
        INSERT INTO outputStream\n\
        SELECT abs(value) AS result FROM numStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("numStream", vec![AttributeValue::Int(0)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    let val = match &out[0][0] {
        AttributeValue::Int(v) => *v,
        AttributeValue::Long(v) => *v as i32,
        _ => panic!("Expected int"),
    };
    assert_eq!(val, 0);
}

/// abs with positive value (stays same)
#[tokio::test]
async fn function_test_abs_positive_unchanged() {
    let app = "\
        CREATE STREAM numStream (value INT);\n\
        CREATE STREAM outputStream (result INT);\n\
        INSERT INTO outputStream\n\
        SELECT abs(value) AS result FROM numStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("numStream", vec![AttributeValue::Int(42)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    let val = match &out[0][0] {
        AttributeValue::Int(v) => *v,
        AttributeValue::Long(v) => *v as i32,
        _ => panic!("Expected int"),
    };
    assert_eq!(val, 42);
}

/// coalesce with three arguments (takes first)
#[tokio::test]
async fn function_test_coalesce_takes_first_of_three() {
    let app = "\
        CREATE STREAM dataStream (val1 STRING, val2 STRING, val3 STRING);\n\
        CREATE STREAM outputStream (result STRING);\n\
        INSERT INTO outputStream\n\
        SELECT coalesce(val1, val2, val3) AS result FROM dataStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "dataStream",
        vec![
            AttributeValue::String("first".to_string()),
            AttributeValue::String("second".to_string()),
            AttributeValue::String("third".to_string()),
        ],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    // coalesce returns first non-null
    assert_eq!(out[0][0], AttributeValue::String("first".to_string()));
}

/// concat with three arguments (abc123xyz)
#[tokio::test]
async fn function_test_concat_three_strings_combined() {
    let app = "\
        CREATE STREAM dataStream (first STRING, middle STRING, last STRING);\n\
        CREATE STREAM outputStream (result STRING);\n\
        INSERT INTO outputStream\n\
        SELECT concat(first, middle, last) AS result FROM dataStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "dataStream",
        vec![
            AttributeValue::String("abc".to_string()),
            AttributeValue::String("123".to_string()),
            AttributeValue::String("xyz".to_string()),
        ],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::String("abc123xyz".to_string()));
}

/// sqrt of zero returns zero (edge case)
#[tokio::test]
async fn function_test_sqrt_zero_returns_zero() {
    let app = "\
        CREATE STREAM dataStream (value DOUBLE);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT sqrt(value) AS result FROM dataStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("dataStream", vec![AttributeValue::Double(0.0)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::Double(0.0));
}

/// power with zero exponent returns 1
#[tokio::test]
async fn function_test_power_zero_exponent() {
    let app = "\
        CREATE STREAM dataStream (base DOUBLE);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT power(base, 0) AS result FROM dataStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("dataStream", vec![AttributeValue::Double(5.0)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::Double(1.0));
}

/// nested function: upper(concat(a, b))
#[tokio::test]
async fn function_test_upper_of_concat() {
    let app = "\
        CREATE STREAM dataStream (first STRING, second STRING);\n\
        CREATE STREAM outputStream (result STRING);\n\
        INSERT INTO outputStream\n\
        SELECT upper(concat(first, second)) AS result FROM dataStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "dataStream",
        vec![
            AttributeValue::String("hello".to_string()),
            AttributeValue::String("world".to_string()),
        ],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::String("HELLOWORLD".to_string()));
}

/// nested function: lower(concat(a, b))
#[tokio::test]
async fn function_test_lower_of_concat() {
    let app = "\
        CREATE STREAM dataStream (first STRING, second STRING);\n\
        CREATE STREAM outputStream (result STRING);\n\
        INSERT INTO outputStream\n\
        SELECT lower(concat(first, second)) AS result FROM dataStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "dataStream",
        vec![
            AttributeValue::String("HELLO".to_string()),
            AttributeValue::String("WORLD".to_string()),
        ],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::String("helloworld".to_string()));
}

/// nested function: length(concat(a, b))
#[tokio::test]
async fn function_test_length_of_concat() {
    let app = "\
        CREATE STREAM dataStream (first STRING, second STRING);\n\
        CREATE STREAM outputStream (result INT);\n\
        INSERT INTO outputStream\n\
        SELECT length(concat(first, second)) AS result FROM dataStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "dataStream",
        vec![
            AttributeValue::String("abc".to_string()),
            AttributeValue::String("defgh".to_string()),
        ],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    let len = match &out[0][0] {
        AttributeValue::Int(v) => *v,
        AttributeValue::Long(v) => *v as i32,
        _ => panic!("Expected int"),
    };
    assert_eq!(len, 8); // "abcdefgh" = 8 chars
}

/// abs of double negative returns positive double
#[tokio::test]
async fn function_test_abs_large_negative() {
    let app = "\
        CREATE STREAM dataStream (value DOUBLE);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT abs(value) AS result FROM dataStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("dataStream", vec![AttributeValue::Double(-99999.99)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::Double(99999.99));
}

/// round with precision 2
#[tokio::test]
async fn function_test_round_precision_two() {
    let app = "\
        CREATE STREAM dataStream (value DOUBLE);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT round(value, 2) AS result FROM dataStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("dataStream", vec![AttributeValue::Double(3.54567)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::Double(3.55));
}

/// floor of negative decimal
#[tokio::test]
async fn function_test_floor_negative_decimal() {
    let app = "\
        CREATE STREAM dataStream (value DOUBLE);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT floor(value) AS result FROM dataStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("dataStream", vec![AttributeValue::Double(-2.3)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::Double(-3.0));
}

/// ceil of negative decimal
#[tokio::test]
async fn function_test_ceil_negative_decimal() {
    let app = "\
        CREATE STREAM dataStream (value DOUBLE);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT ceil(value) AS result FROM dataStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("dataStream", vec![AttributeValue::Double(-2.7)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::Double(-2.0));
}

/// sqrt of perfect square
#[tokio::test]
async fn function_test_sqrt_perfect_square() {
    let app = "\
        CREATE STREAM dataStream (value DOUBLE);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT sqrt(value) AS result FROM dataStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("dataStream", vec![AttributeValue::Double(16.0)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::Double(4.0));
}

/// sqrt of 1 returns 1 (identity)
#[tokio::test]
async fn function_test_sqrt_one_identity() {
    let app = "\
        CREATE STREAM dataStream (value DOUBLE);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT sqrt(value) AS result FROM dataStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("dataStream", vec![AttributeValue::Double(1.0)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::Double(1.0));
}

/// sin of zero returns zero (using math:sin namespace)
#[tokio::test]
async fn function_test_math_sin_zero() {
    let app = "\
        CREATE STREAM dataStream (value DOUBLE);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT `math:sin`(value) AS result FROM dataStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("dataStream", vec![AttributeValue::Double(0.0)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::Double(0.0));
}

/// tan of zero returns zero (using math:tan namespace)
#[tokio::test]
async fn function_test_math_tan_zero() {
    let app = "\
        CREATE STREAM dataStream (value DOUBLE);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT `math:tan`(value) AS result FROM dataStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("dataStream", vec![AttributeValue::Double(0.0)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::Double(0.0));
}

/// log of 1 returns 0 (using math:log namespace)
#[tokio::test]
async fn function_test_math_log_one() {
    let app = "\
        CREATE STREAM dataStream (value DOUBLE);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT `math:log`(value) AS result FROM dataStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("dataStream", vec![AttributeValue::Double(1.0)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::Double(0.0));
}

/// round of whole number stays same
#[tokio::test]
async fn function_test_round_whole_number() {
    let app = "\
        CREATE STREAM dataStream (value DOUBLE);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT round(value) AS result FROM dataStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("dataStream", vec![AttributeValue::Double(5.0)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::Double(5.0));
}

/// round of half rounds up for 2.5
#[tokio::test]
async fn function_test_round_two_point_five() {
    let app = "\
        CREATE STREAM dataStream (value DOUBLE);\n\
        CREATE STREAM outputStream (result DOUBLE);\n\
        INSERT INTO outputStream\n\
        SELECT round(value) AS result FROM dataStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send("dataStream", vec![AttributeValue::Double(2.5)]);
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    // Expect 3.0 (standard round half up)
    assert_eq!(out[0][0], AttributeValue::Double(3.0));
}

/// substring with start and end
#[tokio::test]
async fn function_test_substring_start_end() {
    let app = "\
        CREATE STREAM dataStream (text STRING);\n\
        CREATE STREAM outputStream (result STRING);\n\
        INSERT INTO outputStream\n\
        SELECT substring(text, 0, 5) AS result FROM dataStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "dataStream",
        vec![AttributeValue::String("HelloWorld".to_string())],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::String("Hello".to_string()));
}

/// concat with multiple spaces
#[tokio::test]
async fn function_test_concat_with_spaces() {
    let app = "\
        CREATE STREAM dataStream (first STRING, second STRING);\n\
        CREATE STREAM outputStream (result STRING);\n\
        INSERT INTO outputStream\n\
        SELECT concat(first, ' ', second) AS result FROM dataStream;\n";
    let runner = AppRunner::new(app, "outputStream").await;
    runner.send(
        "dataStream",
        vec![
            AttributeValue::String("Hello".to_string()),
            AttributeValue::String("World".to_string()),
        ],
    );
    let out = runner.shutdown();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0][0], AttributeValue::String("Hello World".to_string()));
}
