use core::hash;
use core::num;
use csv::StringRecord;
use regex::Regex;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::fs::File;
use std::hash::DefaultHasher;
use std::hash::Hash;
use std::io::{self, Write};
use std::iter::Enumerate;
use std::path::Path;
use std::vec;

mod errors;

mod hashing;
mod regex_serializer;
mod rhai_custom_cell_validator;
use crate::csv_workspace::CachedCsvWorkspace;
use crate::regex_serializer::deserialize_regex_opt;

mod csv_reader;
mod csv_cached;
mod csv_cell_validators;
mod csv_schema;
mod csv_row_validators;
mod util;
mod csv_workspace;


// trait evaluate line
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

impl CellEvaluator for CsvColumnSchema {
    fn eval_value(&self, cell: &str, row_map: &HashMap<String, String>) -> ValueValidationResult {
        let required = self.required.unwrap_or(false);
        let dtype = &self.dtype;

        let value = cell.trim().to_string();

        if value.is_empty() {
            return if required {
                ValueValidationResult::Error {
                    value,
                    errors: vec![ColumnError::RequiredMissing],
                }
            } else {
                ValueValidationResult::Ok { value }
            };
        }

        dtype.eval_value(&value, &row_map)
    }
}


#[derive(Debug)]
enum CellEvalResult {
    Ok {
        value: String,
        col_no: u16,
        line_no: u64,
    },
    Error {
        value: String,
        col_no: u16,
        line_no: u64,
        errors: Vec<ColumnError>,
    },
}



fn write_result(result: Vec<LineEvalResult>, errors_only: bool) {
    for line in result {
        let is_error = matches!(
            line,
            LineEvalResult::Error { .. } | LineEvalResult::LineSkipped { .. }
        );

        if !errors_only || is_error {
            println!("{:?}", line);
            println!("- - - - ");
        }
    }
}

fn build_unique_bitmask(columns: &Vec<CsvColumnSchema>, unique_col_idx: &Vec<usize>) -> Vec<u8> {
    /*
    buils a bit mask vector same size as columns length with value 1 where the column is supposed to be unique and 0 otherwise
     */
    let bit_mask: Vec<u8> = columns
        .iter()
        .enumerate()
        .map(|(idx, _)| {
            if unique_col_idx.iter().any(|x| *x == idx) {
                1
            } else {
                0
            }
        })
        .collect();

    bit_mask
}

fn build_unique_col_idx(
    constraint: &SchemaUniqueSpecifier,
    col_name_2_col_idx: &HashMap<String, usize>,
) -> Vec<usize> {
    /*
    maps the column names to their index
     */
    let unique_col_idx: Vec<usize> = constraint
        .columns
        .iter()
        .map(|col_name| {
            *col_name_2_col_idx.get(col_name).expect(&format!(
                "given unique column name not found in schema: {}",
                col_name
            ))
        })
        .collect();

    unique_col_idx
}


fn build_unique_constraint(
    schema: &CsvSchema,
    constraint: &SchemaUniqueSpecifier,
) -> UniqueConstraint {
    let col_name_2_col_idx: HashMap<String, usize> = schema
        .columns
        .iter()
        .enumerate() // gives (idx, &column)
        .map(|(idx, column)| (column.name.clone(), idx))
        .collect();

    let unique_col_idx = build_unique_col_idx(constraint, &col_name_2_col_idx);
    let bit_mask = build_unique_bitmask(&schema.columns, &unique_col_idx);

    // returns constraint paired with bit_mask
    UniqueConstraint {
        constraint: constraint.clone(),
        bit_mask,
    }
}



fn build_fk_hashmap(rows: Vec<Vec<&str>>, hash_indizes: Vec<usize>) -> HashMap<u64, usize> {
    /*
    iterates over all rows, builds a hash based on hash_indizes and returns the hashmap mapping fk hash to row
     */
    let mut map: HashMap<u64, usize> = HashMap::new();

    if rows.is_empty() {
        return map;
    }

    let highest_idx = *hash_indizes.iter().max().expect("Expected non empty vec");

    assert!(highest_idx < rows.len());

    let row_len = rows[0].len();

    let mut bitmask = vec![false; row_len];

    for i in hash_indizes {
        bitmask[i] = true;
    }

    for (idx, row) in rows.iter().enumerate() {
        // overwrite duplicate hashes. we only need to know if they exist
        let hash_source: Vec<&str> = row
            .iter()
            .enumerate()
            .filter(|(idx, _)| bitmask[*idx])
            .map(|(_, b)| *b)
            .collect();

        let hash = hashing::hash_vec(&hash_source);
        map.insert(hash, idx);
    }

    map
}


fn main() {
    let mut cached_workspace = CachedCsvWorkspace::new();

    cached_workspace.add_csv_file("mt_article.csv", "schema.mt_article.toml");

    println!("{:?}", cached_workspace.files());

    let csv_article = cached_workspace.csv_or_panic("mt_article.csv");
    println!("csv rows len {:?}", csv_article.rows.len());
    input!("added file");
    let csv_path_string = "mt_article.csv";

    let csv_path = Path::new(csv_path_string);

    assert!(
        csv_path.exists() && csv_path.is_file(),
        "csv file not a file {}",
        csv_path_string
    );

    let stem = csv_path
        .file_stem()
        .expect("cant extract stem from filename")
        .to_str()
        .expect("cant convert filename to string");

    let schema_path_string = format!("schema.{}.toml", stem);
    let schema_path = Path::new(&schema_path_string);

    assert!(
        schema_path.exists() && schema_path.is_file(),
        "schema file not a file {}",
        csv_path_string
    );

    let schema = read_csv_schema(&schema_path).expect("Cant read CSV Schema");

    println!("schema.fk :: {:?}", schema.foreign_keys);

    let mut line_eval_results: Vec<LineEvalResult> = Vec::new();

    let unique_constraints: Vec<UniqueConstraint> = schema
        .unique
        .iter()
        .map(|constraint| build_unique_constraint(&schema, constraint))
        .collect();

    // unique row vec maps to index to line eval results
    let mut unique_hashmap: HashMap<u64, usize> = HashMap::new();

    let mut csv_reader = build_csv_reader(csv_path_string, &schema.settings);

    let mut filename2CachedCsv: HashMap<String, CachedCsv> = HashMap::new();

    let mut cashed_csv = CachedCsv {
        filename: csv_path_string.to_string(),
        hash_func: hashing::hash_vec,
        hashed_rows: HashMap::new(),
        rows: read_csv(&mut csv_reader, &schema),
    };

    cashed_csv.hash_rows_for(&[0]);

    // build hashed csv tables linked in foreign keys

    // check foreign key constraints...
    for fk in &schema.foreign_keys {
        // fk.columns

        // fk.references.columns
        let file = &fk.references.file;
    }

    input!("press enter");

    let mut row_cnt: u64 = 0;

    for row in csv_reader.records() {
        row_cnt += 1;

        if let Ok(row) = row {
            // println!("ROW: {:?}", row);

            // skip empty lines
            if row.is_empty() || row.len() == 1 && row[0].is_empty() {
                continue;
            }

            let line_no = row.position().unwrap().line(); //record(); //line();

            // println!("row: {:?}, line_no: {}", row, line_no);
            // continue;

            if row.len() != schema.columns.len() {
                let line_eval_result = LineEvalResult::LineSkipped {
                    line_no,
                    reason: format!(
                        "Invalid row length. expected: {}, actual: {} ",
                        schema.columns.len(),
                        row.len()
                    ),
                };
                line_eval_results.push(line_eval_result);
                continue;
            }

            // trim all cells once upfront
            let trimmed_row: Vec<String> = row.iter().map(|c| c.trim().to_string()).collect();
            // build the named map for rhai scripts
            let row_map: HashMap<String, String> = schema
                .columns
                .iter()
                .enumerate()
                .map(|(idx, col)| (col.name.clone(), trimmed_row[idx].clone()))
                .collect();

            let mut cell_eval_results: Vec<CellEvalResult> = Vec::new();

            let mut has_err = false;

            let unique_constraints_len: usize = unique_constraints.len();
            let mut line_unique_hash_vec_per_unique_constraint: Vec<Vec<&str>> =
                vec![Vec::new(); unique_constraints_len];

            for (idx, column_schema) in schema.columns.iter().enumerate() {
                let cell = &row[idx];

                let col_idx_human = (idx + 1) as u16;

                // do single cell validation
                let eval_result: ValueValidationResult = column_schema.eval_value(cell, &row_map);

                // check foreign key constraints...
                for fk in &schema.foreign_keys {
                    // fk.columns

                    // fk.references.columns
                    // fk.references.file
                }

                // for every unique constraint get bit mask and do check

                for (idx, unique_con) in (&unique_constraints).iter().enumerate() {
                    let bit_mask = &unique_con.bit_mask;

                    let unique_hash_4_constraint =
                        &mut line_unique_hash_vec_per_unique_constraint[idx];

                    if bit_mask[idx] == 1 {
                        // println!(
                        //     "add col to hash form bit mask. {}, {}",
                        //     idx, column_schema.name
                        // );

                        unique_hash_4_constraint.push(cell);
                    }
                }

                if matches!(eval_result, ValueValidationResult::Error { .. }) {
                    has_err = true;
                }

                let cell_eval_result = match eval_result {
                    ValueValidationResult::Ok { value } => CellEvalResult::Ok {
                        value: value,
                        col_no: col_idx_human,
                        line_no: line_no,
                    },
                    ValueValidationResult::Error { value, errors } => CellEvalResult::Error {
                        value,
                        col_no: col_idx_human,
                        errors,
                        line_no,
                    },
                };
                cell_eval_results.push(cell_eval_result);
            }

            // check unique constraints

            let mut unique_violation: Vec<UniqueViolation> = Vec::new();

            let mut all_hash_values: Vec<u64> = Vec::new();

            for (idx, line_unique_hash_vec) in line_unique_hash_vec_per_unique_constraint
                .iter()
                .enumerate()
            {
                let hash_val = hashing::hash_vec(&line_unique_hash_vec);

                all_hash_values.push(hash_val);

                if unique_hashmap.contains_key(&hash_val) {
                    let violated_constraint = &unique_constraints[idx];
                    let line_eval_res_idx = unique_hashmap[&hash_val];
                    let other = &line_eval_results[line_eval_res_idx];

                    // println!(
                    //     "HASH already SEEN. Unique Constraint violated: {:#?},  {:?}, line_no: {}, other line_no: {}",
                    //     violated_constraint.constraint.name,
                    //     violated_constraint.constraint.columns,
                    //     line_no,
                    //     other.line_no()
                    // );

                    unique_violation.push(UniqueViolation {
                        line_no: other.line_no(),
                        constraint: violated_constraint.clone(),
                    });

                    has_err = true;
                }
            }

            let line_eval_result = if has_err {
                LineEvalResult::Error {
                    line_no,
                    cell_eval_results: cell_eval_results,
                    unique_violations: unique_violation,
                }
            } else {
                LineEvalResult::Ok {
                    line_no,
                    cell_eval_results: cell_eval_results,
                }
            };

            line_eval_results.push(line_eval_result);
            // store index to line eval result

            for hash_val in all_hash_values {
                unique_hashmap.insert(hash_val, line_eval_results.len() - 1);
            }
        } else {
            println!("Cant read row {:?}", row);
        }
    }

    println!(
        "lines evaluated: {} :: {}",
        line_eval_results.len(),
        row_cnt
    );
    write_result(line_eval_results, true);
}
