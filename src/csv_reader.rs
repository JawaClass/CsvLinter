use crate::{
    csv_row_validators::ColumnSelection, csv_schema::CsvColumnSchema, errors::CsvSchemaError,
    hashing,
};
use std::{alloc::System, collections::HashMap, path::Path, str, time::SystemTime};

use crate::csv_schema::{CsvSchema, CsvSchemaSettings};

#[derive(Debug)]
pub enum CsvRowError {
    WrongColumnCount { expected: usize, actual: usize },
    ParseError(String),
    InvalidUtf8,
    // add more as needed
}

#[derive(Debug)]
pub struct ErrorRow {
    pub line_no: u64,
    pub reason: CsvRowError,
}

#[derive(Debug)]
pub struct OkRow {
    pub line_no: u64,
    pub cells: Vec<String>,
    pub index: usize,
}

impl OkRow {
    pub fn hash(&self, column_selection: &ColumnSelection) -> u64 {
        let row_len = self.cells.len();

        let mut bitmask = vec![false; row_len];

        for i in column_selection {
            bitmask[*i] = true;
        }

        let hash_source: Vec<&str> = self
            .cells
            .iter()
            .enumerate()
            .filter(|(idx, _)| bitmask[*idx])
            .map(|(_, cell)| cell.as_str())
            .collect();

        let hash = hashing::hash_vec(&hash_source);
        hash
    }

    pub fn as_hashmap(&self, schema: &Vec<CsvColumnSchema>) -> HashMap<String, String> {
        assert!(self.cells.len() == schema.len());
        self.cells
            .iter()
            .enumerate()
            .map(|(idx, cell)| (schema[idx].name.clone(), cell.clone()))
            .collect::<HashMap<String, String>>()
    }
}

fn csv_record_to_vec(row: &csv::StringRecord) -> Vec<String> {
    /*
    returns a vec of trimmed cell values from a csv string record
     */
    row.iter().map(|c| c.trim().to_string()).collect()
}

pub struct CsvReadResult {
    pub ok_rows: Vec<OkRow>,
    pub error_rows: Vec<ErrorRow>,
    pub timestamp: SystemTime,
}

impl CsvReadResult {
    pub fn from_reader(reader: &mut csv::Reader<std::fs::File>, schema: &CsvSchema) -> Self {
        read_csv(reader, schema)
    }

    pub fn total_rows(&self) -> usize {
        self.ok_rows.len() + self.error_rows.len()
    }
}

pub fn read_csv(reader: &mut csv::Reader<std::fs::File>, schema: &CsvSchema) -> CsvReadResult {
    /* reads the csv and returns a struct with successfuly parsed rows and error rows */
    let expected_col_len = schema.columns.len();
    let mut ok_rows = Vec::new();
    let mut error_rows = Vec::new();

    println!("read_csv................... {:?}", schema.columns);

    for record in reader.records() {
        match record {
            Ok(row) => {
                let line_no = row.position().unwrap().line();
                let row = csv_record_to_vec(&row);

                if row.len() != expected_col_len {
                    let result = ErrorRow {
                        line_no,
                        reason: CsvRowError::WrongColumnCount {
                            expected: expected_col_len,
                            actual: row.len(),
                        },
                    };
                    error_rows.push(result);
                } else {
                    let result = OkRow {
                        line_no,
                        cells: row,
                        index: ok_rows.len(),
                    };
                    ok_rows.push(result);
                }
            }
            Err(err) => {
                // not all errors have position line no
                let line_no = err.position().map(|v| v.line()).unwrap_or(0);
                let result = ErrorRow {
                    line_no,
                    reason: CsvRowError::ParseError(err.to_string()),
                };
                error_rows.push(result);
            }
        }
    }

    CsvReadResult {
        ok_rows,
        error_rows,
        timestamp: SystemTime::now(),
    }
}

#[derive(thiserror::Error, Debug)]
pub enum CsvReadError {
    #[error("Can not open CSV file.")]
    Io(#[from] std::io::Error),
}
pub fn build_csv_reader(
    file_path: &str,
    settings: &CsvSchemaSettings,
) -> Result<csv::Reader<std::fs::File>, CsvReadError> {
    /*
     * create a csv reader with custom settings
     * encoding needs to be handlet later because csv reader only supports utf8
     */
    let delim = settings.delimiter.as_bytes()[0];

    let comment = settings.comment.as_bytes()[0];

    let file = std::fs::File::open(file_path)?;

    Ok(csv::ReaderBuilder::new()
        .has_headers(false)
        .delimiter(delim)
        .double_quote(false)
        .escape(Some(b'\\'))
        // .flexible(true)
        .comment(Some(comment))
        .from_reader(file))
}
