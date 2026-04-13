use std::path::Path;

use serde::{Deserialize, Serialize};

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

#[derive(Clone, Debug, Deserialize)] // serde: parse FROM a format (toml, json, etc.)
#[derive(Serialize)] // serde: convert TO a format
pub struct SchemaUniqueSpecifier {
    columns: Vec<String>,
    name: String,
}

#[derive(Debug, Deserialize)]
pub struct CsvColumnSchema {
    name: String,
    #[serde(default)]
    required: Option<bool>,
    #[serde(flatten)]
    dtype: ColumnType,
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
