use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::vec;

use serde::{Deserialize, Serialize};

use crate::csv_cell_validators::{
    ValueValidationResult, eval_enum, eval_float, eval_integer, eval_string,
};
use crate::csv_row_validators::ColumnSelection;
use crate::regex_serializer::deserialize_regex_opt;

use crate::errors::CsvSchemaError;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ForeignKey {
    pub columns: Vec<String>,
    pub references: ForeignReference,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ForeignReference {
    pub file: String,         // the CSV file being referenced
    pub columns: Vec<String>, // the column in that CSV
}

#[derive(Debug, Deserialize, Clone)]
pub struct CsvSchemaSettings {
    pub comment: String,
    pub encoding: String,
    pub delimiter: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CsvSchema {
    pub columns: Vec<CsvColumnSchema>,
    // if toml has no unique entry initialize with empty Vector
    #[serde(default)]
    pub unique: Vec<SchemaUniqueConstraint>,
    // if toml has no foreign_keys entry initialize with empty Vector
    #[serde(default)]
    pub foreign_keys: Vec<ForeignKey>,
    pub settings: CsvSchemaSettings,
}

impl CsvSchema {
    pub fn named_col2col_selection(&self, named_columns: &Vec<String>) -> ColumnSelection {
        named_columns
            .iter()
            .map(|col_name| {
                self.col_name_to_col_idx(col_name).unwrap_or_else(|| {
                    panic!(
                        "Column '{}' not found in {:?}",
                        col_name,
                        self.columns
                            .iter()
                            .map(|col| col.name.clone())
                            .collect::<Vec<String>>()
                    )
                })
            })
            .collect()
    }

    fn col_name_to_col_idx(&self, col_name: &str) -> Option<usize> {
        for (col_idx, col) in self.columns.iter().enumerate() {
            if col.name == *col_name {
                return Some(col_idx);
            }
        }
        None
    }
}

#[derive(Clone, Debug, Deserialize)] // serde: parse FROM a format (toml, json, etc.)
#[derive(Serialize)] // serde: convert TO a format
pub struct SchemaUniqueConstraint {
    pub columns: Vec<String>,
    pub name: String,
}

pub trait Conditional {
    fn evaluate(&self, row: &HashMap<String, String>) -> bool;
    // fn columns_referenced(&self) -> Vec<&str>;
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Leaf {
    pub column: String,
    #[serde(flatten)]
    pub op: Op,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Or {
    pub or: Vec<CsvCondition>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct And {
    pub and: Vec<CsvCondition>,
}

impl Conditional for Leaf {
    fn evaluate(&self, row: &HashMap<String, String>) -> bool {
        let column = &self.column;
        let cell = row.get(column).unwrap();
        match &self.op {
            Op::Eq(value) => cell == value,
        }
    }
}

impl Conditional for Or {
    fn evaluate(&self, row: &HashMap<String, String>) -> bool {
        self.or.iter().any(|c| c.evaluate(row))
    }
}

impl Conditional for And {
    fn evaluate(&self, row: &HashMap<String, String>) -> bool {
        self.and.iter().all(|c| c.evaluate(row))
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum CsvCondition {
    Leaf(Leaf),
    And(And),
    Or(Or),
}

impl Conditional for CsvCondition {
    fn evaluate(&self, row: &HashMap<String, String>) -> bool {
        match self {
            CsvCondition::Leaf(l) => l.evaluate(row),
            CsvCondition::Or(o) => o.evaluate(row),
            CsvCondition::And(a) => a.evaluate(row),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Op {
    #[serde(rename = "eq")]
    Eq(String),
    // add more ops here: ne, lt, gt, etc.
}

#[derive(Debug, Deserialize, Clone)]
pub struct CsvColumnSchemaOverride {
    #[serde(default)]
    pub required: Option<bool>,
    #[serde(flatten)]
    pub dtype: ColumnType,
    pub condition: CsvCondition,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CsvColumnSchema {
    pub name: String,
    #[serde(default)]
    pub required: Option<bool>,
    #[serde(flatten)]
    pub dtype: ColumnType,
    // overrides
    pub overrides: Option<Vec<CsvColumnSchemaOverride>>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "dtype")]
pub enum ColumnType {
    #[serde(rename = "string")]
    String {
        min_length: Option<u32>,
        max_length: Option<u32>,
        // INFO: without default param an Option cant be desierialied with custom function
        #[serde(default, deserialize_with = "deserialize_regex_opt")]
        regex: Option<regex::Regex>,
        custom_validator: Option<String>,
    },
    #[serde(rename = "integer")]
    Integer { min: Option<i64>, max: Option<i64> },
    #[serde(rename = "float")]
    Float { min: Option<f64>, max: Option<f64> },
    #[serde(rename = "enum")]
    Enum { values: Vec<String> },
}

pub trait CellEvaluator {
    fn eval_value(&self, value: &str, row_map: &HashMap<String, String>) -> ValueValidationResult;
}

impl CellEvaluator for ColumnType {
    fn eval_value(&self, cell: &str, row_map: &HashMap<String, String>) -> ValueValidationResult {
        match self {
            ColumnType::Integer { min, max } => {
                return eval_integer(cell, min, max);
            }
            ColumnType::Float { min, max } => {
                return eval_float(cell, min, max);
            }
            ColumnType::String {
                min_length,
                max_length,
                regex,
                custom_validator,
            } => {
                return eval_string(
                    cell,
                    min_length,
                    max_length,
                    regex,
                    custom_validator,
                    row_map,
                );
            }
            ColumnType::Enum { values } => {
                return eval_enum(cell, values);
            }
        }
    }
}

// impl CellEvaluator for CsvColumnSchema {
//     fn eval_value(&self, cell: &str, row_map: &HashMap<String, String>) -> ValueValidationResult {
//         let required = self.required.unwrap_or(false);
//         let dtype = &self.dtype;

//         let value = cell.trim().to_string();

//         if value.is_empty() {
//             return if required {
//                 ValueValidationResult::Error {
//                     value,
//                     errors: vec![ColumnError::RequiredMissing],
//                 }
//             } else {
//                 ValueValidationResult::Ok { value }
//             };
//         }

//         dtype.eval_value(&value, &row_map)
//     }
// }

pub fn read_csv_schema(path: &Path) -> Result<CsvSchema, CsvSchemaError> {
    assert!(
        path.exists() && path.is_file(),
        "{}",
        format!("Cant read csv schema from file: {:?}", path)
    );
    let content = std::fs::read_to_string(path)?;
    let schema: CsvSchema = toml::from_str(&content)?;

    Ok(schema)
}
