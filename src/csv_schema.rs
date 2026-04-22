use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::vec;

use serde::{Deserialize, Serialize};

use crate::csv_cell_validators::{
    ValueValidationResult, eval_enum, eval_float, eval_integer, eval_string,
};
use crate::regex_serializer::deserialize_regex_opt;

use crate::errors::CsvSchemaError;

#[derive(Debug, Deserialize, Serialize)]
pub struct ForeignKey {
    columns: Vec<String>,
    references: ForeignReference,
    name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ForeignReference {
    file: String,         // the CSV file being referenced
    columns: Vec<String>, // the column in that CSV
}

#[derive(Debug, Deserialize)]
pub struct CsvSchemaSettings {
    pub comment: String,
    pub encoding: String,
    pub delimiter: String,
}

#[derive(Debug, Deserialize)]
pub struct CsvSchema {
    pub columns: Vec<CsvColumnSchema>,
    // if toml has no unique entry initialize with empty Vector
    #[serde(default)]
    pub unique: Vec<SchemaUniqueSpecifier>,
    // if toml has no foreign_keys entry initialize with empty Vector
    #[serde(default)]
    pub foreign_keys: Vec<ForeignKey>,
    pub settings: CsvSchemaSettings,
}

impl CsvSchema {
    pub fn unique_constraints(&self) -> Vec<Vec<usize>> {
        let unique_names: HashSet<&str> = self.unique.iter().map(|s| s.name.as_str()).collect();
        let all_unique = unique_names.len() == self.unique.len();
        if !all_unique {
            panic!("unique constraints have duplicate names!");
        }

        let ret: Vec<Vec<usize>> = self
            .unique
            .iter()
            .map(|constraint| self.eval_indizes_unique_constraint(constraint))
            .collect();

        ret
    }

    fn col_name_to_col_idx(&self, col_name: &str) -> Option<usize> {
        for (col_idx, col) in self.columns.iter().enumerate() {
            if col.name == *col_name {
                return Some(col_idx);
            }
        }
        None
    }

    pub fn eval_indizes_unique_constraint(&self, constraint: &SchemaUniqueSpecifier) -> Vec<usize> {
        let indizes = constraint
            .columns
            .iter()
            .map(|c| {
                self.col_name_to_col_idx(c)
                    .expect("Unique Col Name does not exist")
            })
            .collect();

        indizes
    }
}

#[derive(Clone, Debug, Deserialize)] // serde: parse FROM a format (toml, json, etc.)
#[derive(Serialize)] // serde: convert TO a format
pub struct SchemaUniqueSpecifier {
    pub columns: Vec<String>,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct CsvColumnSchema {
    pub name: String,
    #[serde(default)]
    pub required: Option<bool>,
    #[serde(flatten)]
    pub dtype: ColumnType,
}

#[derive(Debug, Deserialize)]
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
