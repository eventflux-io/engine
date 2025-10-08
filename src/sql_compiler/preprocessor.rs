// SPDX-License-Identifier: MIT OR Apache-2.0

//! SQL Preprocessor - Extract Streaming Extensions
//!
//! **DEPRECATED (v0.1.2 - 2025-10-08)**: This regex-based preprocessor has been replaced
//! by native SQL parser integration. The forked sqlparser now directly supports
//! `WINDOW()` syntax in the AST via `StreamingWindowSpec`.
//!
//! This module is maintained for backward compatibility but is no longer used internally.
//! New code should use the native parser in `converter.rs`.
//!
//! See FORK_MAINTENANCE.md and feat/grammar/GRAMMAR.md for details.

use once_cell::sync::Lazy;
use regex::Regex;
use sqlparser::ast::Expr as SqlExpr;

use super::error::PreprocessorError;

/// Regex to match WINDOW clause syntax: WINDOW('type', params)
static WINDOW_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?i)\bWINDOW\s*\(\s*'(\w+)'\s*,\s*(.+?)\s*\)"#)
        .expect("Invalid WINDOW regex")
});

/// Time unit for window specifications
#[derive(Debug, Clone, PartialEq)]
pub enum TimeUnit {
    Milliseconds,
    Seconds,
    Minutes,
    Hours,
}

/// Window specification extracted from SQL
#[derive(Debug, Clone, PartialEq)]
pub enum WindowSpec {
    Tumbling {
        value: i64,
        unit: TimeUnit,
    },
    Sliding {
        window_value: i64,
        window_unit: TimeUnit,
        slide_value: i64,
        slide_unit: TimeUnit,
    },
    Length {
        size: i64,
    },
    Session {
        value: i64,
        unit: TimeUnit,
    },
    Time {
        value: i64,
        unit: TimeUnit,
    },
    TimeBatch {
        value: i64,
        unit: TimeUnit,
    },
    LengthBatch {
        size: i64,
    },
    ExternalTime {
        timestamp_field: String,
        value: i64,
        unit: TimeUnit,
    },
    ExternalTimeBatch {
        timestamp_field: String,
        value: i64,
        unit: TimeUnit,
    },
}

/// Text representation of window clause
#[derive(Debug, Clone)]
pub struct WindowClauseText {
    pub full_match: String,
    pub window_type: String,
    pub parameters: String,
    pub spec: WindowSpec,
}

/// Preprocessed SQL with extracted components
#[derive(Debug, Clone)]
pub struct PreprocessedSql {
    pub standard_sql: String,
    pub window_clause: Option<WindowClauseText>,
}

/// SQL Preprocessor
pub struct SqlPreprocessor;

impl SqlPreprocessor {
    /// Preprocess SQL to extract streaming extensions
    pub fn preprocess(sql: &str) -> Result<PreprocessedSql, PreprocessorError> {
        let mut result = PreprocessedSql {
            standard_sql: sql.to_string(),
            window_clause: None,
        };

        // Parse WINDOW('type', params) syntax
        if let Some(captures) = WINDOW_REGEX.captures(sql) {
            let full_match = captures
                .get(0)
                .ok_or_else(|| PreprocessorError::WindowParseFailed("No match found".to_string()))?
                .as_str()
                .to_string();

            let window_type = captures
                .get(1)
                .ok_or_else(|| PreprocessorError::WindowParseFailed("No window type".to_string()))?
                .as_str()
                .to_lowercase();

            let parameters = captures
                .get(2)
                .ok_or_else(|| PreprocessorError::WindowParseFailed("No parameters".to_string()))?
                .as_str()
                .trim()
                .to_string();

            // Parse window specification
            let spec = Self::parse_window_spec(&window_type, &parameters)?;

            result.window_clause = Some(WindowClauseText {
                full_match: full_match.clone(),
                window_type: window_type.clone(),
                parameters,
                spec,
            });

            // Remove WINDOW clause from SQL
            result.standard_sql = sql.replace(&full_match, "").trim().to_string();
        }

        Ok(result)
    }

    /// Parse window specification: WINDOW('type', params)
    fn parse_window_spec(window_type: &str, params: &str) -> Result<WindowSpec, PreprocessorError> {
        match window_type {
            "tumbling" => {
                // WINDOW('tumbling', INTERVAL '5' SECOND)
                // or WINDOW('tumbling', size=INTERVAL '5' SECOND)
                let time_param = if params.contains('=') {
                    // Named parameter: size=INTERVAL...
                    let parts: Vec<&str> = params.split('=').collect();
                    if parts.len() >= 2 {
                        parts[1].trim()
                    } else {
                        params.trim()
                    }
                } else {
                    params.trim()
                };
                let (value, unit) = Self::parse_time_param(time_param)?;
                Ok(WindowSpec::Tumbling { value, unit })
            }
            "sliding" | "hop" => {
                // WINDOW('sliding', size=INTERVAL '1' HOUR, slide=INTERVAL '15' MINUTE)
                // or WINDOW('sliding', INTERVAL '1' HOUR, INTERVAL '15' MINUTE)
                let parts: Vec<&str> = params.split(',').map(|s| s.trim()).collect();

                if parts.is_empty() {
                    return Err(PreprocessorError::InvalidWindowParams(
                        "SLIDING window requires size and slide parameters".to_string(),
                    ));
                }

                let (window_value, window_unit, slide_value, slide_unit) = if params.contains('=') {
                    // Named parameters
                    let mut window_param = None;
                    let mut slide_param = None;

                    for part in parts {
                        if part.starts_with("size") {
                            window_param = Some(part.split('=').nth(1).unwrap_or("").trim());
                        } else if part.starts_with("slide") {
                            slide_param = Some(part.split('=').nth(1).unwrap_or("").trim());
                        }
                    }

                    let window_str = window_param.ok_or_else(|| {
                        PreprocessorError::InvalidWindowParams("Missing 'size' parameter".to_string())
                    })?;
                    let slide_str = slide_param.ok_or_else(|| {
                        PreprocessorError::InvalidWindowParams("Missing 'slide' parameter".to_string())
                    })?;

                    let (wv, wu) = Self::parse_time_param(window_str)?;
                    let (sv, su) = Self::parse_time_param(slide_str)?;
                    (wv, wu, sv, su)
                } else {
                    // Positional parameters
                    if parts.len() < 2 {
                        return Err(PreprocessorError::InvalidWindowParams(
                            "SLIDING window requires 2 parameters (size, slide)".to_string(),
                        ));
                    }
                    let (wv, wu) = Self::parse_time_param(parts[0])?;
                    let (sv, su) = Self::parse_time_param(parts[1])?;
                    (wv, wu, sv, su)
                };

                Ok(WindowSpec::Sliding {
                    window_value,
                    window_unit,
                    slide_value,
                    slide_unit,
                })
            }
            "length" => {
                // WINDOW('length', 100)
                // or WINDOW('length', count=100)
                let count_param = if params.contains('=') {
                    let parts: Vec<&str> = params.split('=').collect();
                    if parts.len() >= 2 {
                        parts[1].trim()
                    } else {
                        params.trim()
                    }
                } else {
                    params.trim()
                };
                let size = Self::parse_int_param(count_param)?;
                Ok(WindowSpec::Length { size })
            }
            "session" => {
                // WINDOW('session', INTERVAL '30' SECOND)
                // or WINDOW('session', gap=INTERVAL '30' SECOND)
                let gap_param = if params.contains('=') {
                    let parts: Vec<&str> = params.split('=').collect();
                    if parts.len() >= 2 {
                        parts[1].trim()
                    } else {
                        params.trim()
                    }
                } else {
                    params.trim()
                };
                let (value, unit) = Self::parse_time_param(gap_param)?;
                Ok(WindowSpec::Session { value, unit })
            }
            "time" => {
                // WINDOW('time', INTERVAL '100' MILLISECOND)
                // or WINDOW('time', duration=100)
                let time_param = if params.contains('=') {
                    let parts: Vec<&str> = params.split('=').collect();
                    if parts.len() >= 2 {
                        parts[1].trim()
                    } else {
                        params.trim()
                    }
                } else {
                    params.trim()
                };
                let (value, unit) = Self::parse_time_param(time_param)?;
                Ok(WindowSpec::Time { value, unit })
            }
            "timebatch" => {
                // WINDOW('timeBatch', INTERVAL '100' MILLISECOND)
                // or WINDOW('timeBatch', duration=100)
                let time_param = if params.contains('=') {
                    let parts: Vec<&str> = params.split('=').collect();
                    if parts.len() >= 2 {
                        parts[1].trim()
                    } else {
                        params.trim()
                    }
                } else {
                    params.trim()
                };
                let (value, unit) = Self::parse_time_param(time_param)?;
                Ok(WindowSpec::TimeBatch { value, unit })
            }
            "lengthbatch" => {
                // WINDOW('lengthBatch', 2)
                // or WINDOW('lengthBatch', count=2)
                let count_param = if params.contains('=') {
                    let parts: Vec<&str> = params.split('=').collect();
                    if parts.len() >= 2 {
                        parts[1].trim()
                    } else {
                        params.trim()
                    }
                } else {
                    params.trim()
                };
                let size = Self::parse_int_param(count_param)?;
                Ok(WindowSpec::LengthBatch { size })
            }
            "externaltime" => {
                // WINDOW('externalTime', ts, INTERVAL '100' MILLISECOND)
                // or WINDOW('externalTime', timestamp=ts, duration=INTERVAL '100' MILLISECOND)
                let parts: Vec<&str> = params.split(',').map(|s| s.trim()).collect();

                if parts.is_empty() {
                    return Err(PreprocessorError::InvalidWindowParams(
                        "externalTime window requires timestamp field and duration parameters".to_string(),
                    ));
                }

                let (timestamp_field, time_param) = if params.contains('=') {
                    // Named parameters
                    let mut ts_field = None;
                    let mut duration_param = None;

                    for part in parts {
                        if part.starts_with("timestamp") {
                            ts_field = Some(part.split('=').nth(1).unwrap_or("").trim());
                        } else if part.starts_with("duration") {
                            duration_param = Some(part.split('=').nth(1).unwrap_or("").trim());
                        }
                    }

                    let ts = ts_field.ok_or_else(|| {
                        PreprocessorError::InvalidWindowParams("Missing 'timestamp' parameter".to_string())
                    })?;
                    let dur = duration_param.ok_or_else(|| {
                        PreprocessorError::InvalidWindowParams("Missing 'duration' parameter".to_string())
                    })?;
                    (ts.to_string(), dur)
                } else {
                    // Positional parameters
                    if parts.len() < 2 {
                        return Err(PreprocessorError::InvalidWindowParams(
                            "externalTime window requires 2 parameters (timestamp, duration)".to_string(),
                        ));
                    }
                    (parts[0].to_string(), parts[1])
                };

                let (value, unit) = Self::parse_time_param(time_param)?;
                Ok(WindowSpec::ExternalTime { timestamp_field, value, unit })
            }
            "externaltimebatch" => {
                // WINDOW('externalTimeBatch', ts, INTERVAL '100' MILLISECOND)
                // or WINDOW('externalTimeBatch', timestamp=ts, duration=INTERVAL '100' MILLISECOND)
                let parts: Vec<&str> = params.split(',').map(|s| s.trim()).collect();

                if parts.is_empty() {
                    return Err(PreprocessorError::InvalidWindowParams(
                        "externalTimeBatch window requires timestamp field and duration parameters".to_string(),
                    ));
                }

                let (timestamp_field, time_param) = if params.contains('=') {
                    // Named parameters
                    let mut ts_field = None;
                    let mut duration_param = None;

                    for part in parts {
                        if part.starts_with("timestamp") {
                            ts_field = Some(part.split('=').nth(1).unwrap_or("").trim());
                        } else if part.starts_with("duration") {
                            duration_param = Some(part.split('=').nth(1).unwrap_or("").trim());
                        }
                    }

                    let ts = ts_field.ok_or_else(|| {
                        PreprocessorError::InvalidWindowParams("Missing 'timestamp' parameter".to_string())
                    })?;
                    let dur = duration_param.ok_or_else(|| {
                        PreprocessorError::InvalidWindowParams("Missing 'duration' parameter".to_string())
                    })?;
                    (ts.to_string(), dur)
                } else {
                    // Positional parameters
                    if parts.len() < 2 {
                        return Err(PreprocessorError::InvalidWindowParams(
                            "externalTimeBatch window requires 2 parameters (timestamp, duration)".to_string(),
                        ));
                    }
                    (parts[0].to_string(), parts[1])
                };

                let (value, unit) = Self::parse_time_param(time_param)?;
                Ok(WindowSpec::ExternalTimeBatch { timestamp_field, value, unit })
            }
            _ => Err(PreprocessorError::InvalidWindowType(
                window_type.to_string(),
            )),
        }
    }

    /// Parse time parameter (supports seconds, milliseconds, etc.)
    /// Returns (value, unit) tuple
    fn parse_time_param(param: &str) -> Result<(i64, TimeUnit), PreprocessorError> {
        let param = param.trim();

        // Handle INTERVAL syntax: INTERVAL '5' SECOND
        if param.to_uppercase().starts_with("INTERVAL") {
            let parts: Vec<&str> = param.split_whitespace().collect();
            if parts.len() >= 3 {
                let value_str = parts[1].trim_matches('\'').trim_matches('"');
                let unit_str = parts[2].to_uppercase();
                let value: i64 = value_str.parse().map_err(|_| {
                    PreprocessorError::InvalidWindowParams(format!("Invalid number: {}", value_str))
                })?;

                let unit = match unit_str.as_str() {
                    "MILLISECOND" | "MILLISECONDS" => TimeUnit::Milliseconds,
                    "SECOND" | "SECONDS" => TimeUnit::Seconds,
                    "MINUTE" | "MINUTES" => TimeUnit::Minutes,
                    "HOUR" | "HOURS" => TimeUnit::Hours,
                    _ => {
                        return Err(PreprocessorError::InvalidWindowParams(format!(
                            "Unknown time unit: {}",
                            unit_str
                        )))
                    }
                };

                return Ok((value, unit));
            }
        }

        // Handle direct numeric values (assume milliseconds)
        if let Ok(num) = param.parse::<i64>() {
            return Ok((num, TimeUnit::Milliseconds));
        }

        Err(PreprocessorError::InvalidWindowParams(format!(
            "Cannot parse time: {}",
            param
        )))
    }

    /// Parse integer parameter
    fn parse_int_param(param: &str) -> Result<i64, PreprocessorError> {
        param.trim().parse::<i64>().map_err(|_| {
            PreprocessorError::InvalidWindowParams(format!("Invalid integer: {}", param))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preprocess_no_window() {
        let sql = "SELECT * FROM stream";
        let result = SqlPreprocessor::preprocess(sql).unwrap();
        assert_eq!(result.standard_sql, sql);
        assert!(result.window_clause.is_none());
    }

    #[test]
    fn test_parse_time_milliseconds() {
        let result = SqlPreprocessor::parse_time_param("INTERVAL '100' MILLISECOND").unwrap();
        assert_eq!(result, (100, TimeUnit::Milliseconds));
    }

    #[test]
    fn test_parse_time_seconds() {
        let result = SqlPreprocessor::parse_time_param("INTERVAL '5' SECOND").unwrap();
        assert_eq!(result, (5, TimeUnit::Seconds));
    }

    #[test]
    fn test_parse_time_minutes() {
        let result = SqlPreprocessor::parse_time_param("INTERVAL '2' MINUTE").unwrap();
        assert_eq!(result, (2, TimeUnit::Minutes));
    }

    #[test]
    fn test_parse_direct_number() {
        let result = SqlPreprocessor::parse_time_param("5000").unwrap();
        assert_eq!(result, (5000, TimeUnit::Milliseconds));
    }

    // ===== WINDOW('type', params) SYNTAX TESTS =====

    #[test]
    fn test_window_syntax_tumbling() {
        let sql = "SELECT * FROM stream WINDOW('tumbling', INTERVAL '5' SECOND)";
        let result = SqlPreprocessor::preprocess(sql).unwrap();

        assert_eq!(result.standard_sql, "SELECT * FROM stream");
        assert!(result.window_clause.is_some());

        let window = result.window_clause.unwrap();
        assert_eq!(window.window_type, "tumbling");
        assert_eq!(
            window.spec,
            WindowSpec::Tumbling {
                value: 5,
                unit: TimeUnit::Seconds
            }
        );
    }

    #[test]
    fn test_window_sliding_positional() {
        let sql = "SELECT * FROM stream WINDOW('sliding', INTERVAL '1' HOUR, INTERVAL '15' MINUTE)";
        let result = SqlPreprocessor::preprocess(sql).unwrap();

        let window = result.window_clause.unwrap();
        assert_eq!(window.window_type, "sliding");
        assert_eq!(
            window.spec,
            WindowSpec::Sliding {
                window_value: 1,
                window_unit: TimeUnit::Hours,
                slide_value: 15,
                slide_unit: TimeUnit::Minutes
            }
        );
    }

    #[test]
    fn test_window_sliding_named() {
        let sql = "SELECT * FROM stream WINDOW('sliding', size=INTERVAL '1' HOUR, slide=INTERVAL '15' MINUTE)";
        let result = SqlPreprocessor::preprocess(sql).unwrap();

        let window = result.window_clause.unwrap();
        assert_eq!(window.window_type, "sliding");
        assert_eq!(
            window.spec,
            WindowSpec::Sliding {
                window_value: 1,
                window_unit: TimeUnit::Hours,
                slide_value: 15,
                slide_unit: TimeUnit::Minutes
            }
        );
    }

    #[test]
    fn test_window_hop() {
        // 'hop' is an alias for 'sliding'
        let sql = "SELECT * FROM stream WINDOW('hop', INTERVAL '5' MINUTE, INTERVAL '1' MINUTE)";
        let result = SqlPreprocessor::preprocess(sql).unwrap();

        let window = result.window_clause.unwrap();
        assert_eq!(window.window_type, "hop");
        assert_eq!(
            window.spec,
            WindowSpec::Sliding {
                window_value: 5,
                window_unit: TimeUnit::Minutes,
                slide_value: 1,
                slide_unit: TimeUnit::Minutes
            }
        );
    }

    #[test]
    fn test_window_length_simple() {
        let sql = "SELECT * FROM stream WINDOW('length', 100)";
        let result = SqlPreprocessor::preprocess(sql).unwrap();

        let window = result.window_clause.unwrap();
        assert_eq!(window.window_type, "length");
        assert_eq!(window.spec, WindowSpec::Length { size: 100 });
    }

    #[test]
    fn test_window_length_named() {
        let sql = "SELECT * FROM stream WINDOW('length', count=50)";
        let result = SqlPreprocessor::preprocess(sql).unwrap();

        let window = result.window_clause.unwrap();
        assert_eq!(window.window_type, "length");
        assert_eq!(window.spec, WindowSpec::Length { size: 50 });
    }

    #[test]
    fn test_window_session() {
        let sql = "SELECT * FROM stream WINDOW('session', INTERVAL '30' SECOND)";
        let result = SqlPreprocessor::preprocess(sql).unwrap();

        let window = result.window_clause.unwrap();
        assert_eq!(window.window_type, "session");
        assert_eq!(
            window.spec,
            WindowSpec::Session {
                value: 30,
                unit: TimeUnit::Seconds
            }
        );
    }

    #[test]
    fn test_window_session_named() {
        let sql = "SELECT * FROM stream WINDOW('session', gap=INTERVAL '2' MINUTE)";
        let result = SqlPreprocessor::preprocess(sql).unwrap();

        let window = result.window_clause.unwrap();
        assert_eq!(window.window_type, "session");
        assert_eq!(
            window.spec,
            WindowSpec::Session {
                value: 2,
                unit: TimeUnit::Minutes
            }
        );
    }

    #[test]
    fn test_window_tumbling_named() {
        let sql = "SELECT * FROM stream WINDOW('tumbling', size=INTERVAL '10' SECOND)";
        let result = SqlPreprocessor::preprocess(sql).unwrap();

        let window = result.window_clause.unwrap();
        assert_eq!(window.window_type, "tumbling");
        assert_eq!(
            window.spec,
            WindowSpec::Tumbling {
                value: 10,
                unit: TimeUnit::Seconds
            }
        );
    }

}
