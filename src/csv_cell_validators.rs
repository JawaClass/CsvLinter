use std::collections::HashMap;

use crate::rhai_custom_cell_validator;

#[derive(Debug)]
pub enum ValueValidationResult {
    Ok {
        value: String,
    },
    Error {
        value: String,
        errors: Vec<CellValidationError>,
    },
}

#[derive(Debug, Clone)]
pub enum CellValidationError {
    RequiredMissing,
    TooShort { min: u32, actual: usize },
    TooLong { max: u32, actual: usize },
    RegexNoMatch { regex: String, value: String },
    NotAnInteger,
    TooSmallInteger { min: i64, actual: i64 },
    TooLargeInteger { max: i64, actual: i64 },
    TooSmallFloat { min: f64, actual: f64 },
    TooLargeFloat { max: f64, actual: f64 },
    InvalidEnumValue { allowed: Vec<String> },
}

pub fn eval_enum(value: &str, values: &Vec<String>) -> ValueValidationResult {
    let is_valid_enum = values.contains(&value.to_string());

    return if is_valid_enum {
        ValueValidationResult::Ok {
            value: value.to_string(),
        }
    } else {
        ValueValidationResult::Error {
            value: value.to_string(),
            errors: vec![CellValidationError::InvalidEnumValue {
                allowed: values.to_vec(),
            }],
        }
    };
}

pub fn eval_string(
    value: &str,
    min_length: &Option<u32>,
    max_length: &Option<u32>,
    regex: &Option<regex::Regex>,
    custom_validator: &Option<String>,
    row_map: &HashMap<String, String>,
) -> ValueValidationResult {
    let mut errors = Vec::new();

    let len = value.len() as u32;

    if let Some(min_length) = min_length {
        if len < *min_length {
            errors.push(CellValidationError::TooShort {
                min: *min_length,
                actual: len as usize,
            });
        }
    }

    if let Some(max_length) = max_length {
        if len > *max_length {
            errors.push(CellValidationError::TooLong {
                max: *max_length,
                actual: len as usize,
            });
        }
    }

    if let Some(regex) = regex {
        if !regex.is_match(value) {
            errors.push(CellValidationError::RegexNoMatch {
                regex: regex.to_string(),
                value: value.to_string(),
            });
        }
    }

    if let Some(custom_validator) = custom_validator {
        let result =
            rhai_custom_cell_validator::run_custom_validator(custom_validator, value, &row_map);
        // println!("rhai script result: {:?}", result);
    }

    return if errors.is_empty() {
        ValueValidationResult::Ok {
            value: value.to_string(),
        }
    } else {
        ValueValidationResult::Error {
            value: value.to_string(),
            errors,
        }
    };
}

pub fn eval_integer(value: &str, min: &Option<i64>, max: &Option<i64>) -> ValueValidationResult {
    let mut errors = Vec::new();

    match value.parse::<i64>() {
        Ok(number) => {
            if let Some(min) = min {
                if number < *min {
                    errors.push(CellValidationError::TooSmallInteger {
                        min: *min,
                        actual: number,
                    });
                }
            }

            if let Some(max) = max {
                if number > *max {
                    errors.push(CellValidationError::TooLargeInteger {
                        max: *max,
                        actual: number,
                    });
                }
            }
            if errors.is_empty() {
                return ValueValidationResult::Ok {
                    value: value.to_string(),
                };
            } else {
                return ValueValidationResult::Error {
                    value: value.to_string(),
                    errors,
                };
            }
        }
        Err(_) => ValueValidationResult::Error {
            value: value.to_string(),
            errors: vec![CellValidationError::NotAnInteger],
        },
    }
}

pub fn eval_float(value: &str, min: &Option<f64>, max: &Option<f64>) -> ValueValidationResult {
    let mut errors = Vec::new();

    match value.parse::<f64>() {
        Ok(number) => {
            if let Some(min) = min {
                if number < *min {
                    errors.push(CellValidationError::TooSmallFloat {
                        min: *min,
                        actual: number,
                    });
                }
            }

            if let Some(max) = max {
                if number > *max {
                    errors.push(CellValidationError::TooLargeFloat {
                        max: *max,
                        actual: number,
                    });
                }
            }
            if errors.is_empty() {
                return ValueValidationResult::Ok {
                    value: value.to_string(),
                };
            } else {
                return ValueValidationResult::Error {
                    value: value.to_string(),
                    errors,
                };
            }
        }
        Err(_) => ValueValidationResult::Error {
            value: value.to_string(),
            errors: vec![CellValidationError::NotAnInteger],
        },
    }
}
