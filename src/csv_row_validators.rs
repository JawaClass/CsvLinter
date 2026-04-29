use core::hash;
use std::{
    cell::{self, Ref},
    collections::{HashMap, HashSet},
    fmt::format,
    hash::Hash,
    vec,
};

use rhai::config::hashing;
use toml::de::ValueDeserializer;

use crate::{
    csv_cached::CsvCachingError,
    csv_cell_validators::{CellValidationError, ValueValidationResult},
    csv_reader::OkRow,
    csv_schema::{CellEvaluator, CsvSchema, ForeignKey, SchemaUniqueConstraint},
    csv_workspace::CsvMapping,
    hashing::hash_vec,
    util::create_bitmask,
};

#[derive(Debug, Clone)]
pub struct CellDataViolation {
    // Ok {
    //     value: String,
    //     col_no: u16,
    //     line_no: u64,
    // },
    pub value: String,
    pub col_no: u16,
    pub line_no: u64,
    pub errors: Vec<CellValidationError>,
}

#[derive(Clone, Debug)]
pub struct UniqueConstraint {
    constraint: SchemaUniqueConstraint,
    bit_mask: Vec<u8>,
}

#[derive(Debug)]
pub struct ForeignKeyViolation {
    pub line_no: u64,
    // pub constraint_name: String,
    pub foreign_key: ForeignKey,
}

#[derive(Debug)]
pub struct UniqueViolation {
    pub line_no: u64,
    pub constraint_name: String,
}

#[derive(Debug)]
pub enum LineSkippedReason {
    ColumnMismatch { expected: u64, actual: u64 },
    UnknownError {},
}
#[derive(Debug)]
pub enum RowCellsValidationResult {
    Ok {
        line_no: u64,
        // cell_validation_results: Vec<CellDataViolation>,
    },
    Skipped {
        line_no: u64,
        reason: LineSkippedReason,
    },
    Error {
        line_no: u64,
        cell_data_violations: Vec<CellDataViolation>,
        fk_violations: Vec<ForeignKeyViolation>, // unique_violations: Vec<UniqueViolation>, // moved to seperate variable
    },
}
// trait evaluate value

impl RowCellsValidationResult {
    fn line_no(&self) -> u64 {
        match self {
            Self::Ok { line_no, .. }
            | Self::Skipped { line_no, .. }
            | Self::Error { line_no, .. } => *line_no,
        }
    }
}

/// The column indices used as input to a hash function
pub type ColumnSelection = Vec<usize>;

/// Maps a hash value to the row index in csv.rows
pub type RowHashMap = HashMap<u64, usize>;

pub struct CsvValidator {
    unique_constraint_rows_seen: HashMap<ColumnSelection, RowHashMap>,
}

impl CsvValidator {
    pub fn new(schema: &CsvSchema) -> CsvValidator {
        let mut seen_map = HashMap::new();

        println!("init CsvValidator...");

        for constraint in &schema.unique {
            let column_selection = schema.named_col2col_selection(&constraint.columns);
            seen_map.insert(column_selection, HashMap::new());
        }

        CsvValidator {
            unique_constraint_rows_seen: seen_map,
        }
    }

    fn build_row_map(&self, row: &OkRow, schema: &CsvSchema) -> HashMap<String, String> {
        let row_map: HashMap<String, String> = schema
            .columns
            .iter()
            .enumerate()
            .map(|(idx, col)| (col.name.clone(), row.cells[idx].clone()))
            .collect();

        row_map
    }

    fn validate_row_cells(&self, schema: &CsvSchema, row: &OkRow) -> Vec<CellDataViolation> {
        // build mapping column name to value
        // required to pass to rhai for custom cell validation
        // TODO: make lazy so it only gets created when rhai gets called...

        let row_map = self.build_row_map(row, schema);

        let mut cell_violations = Vec::new();
        for (col_idx, column_schema) in schema.columns.iter().enumerate() {
            let cell_value = &row.cells[col_idx];

            let result = column_schema.dtype.eval_value(cell_value, &row_map);

            if let ValueValidationResult::Error { value, errors } = &result {
                let result = CellDataViolation {
                    value: value.to_string(),
                    col_no: col_idx as u16,
                    line_no: row.line_no,
                    errors: errors.clone(),
                };
                cell_violations.push(result);
            }
        }

        cell_violations
    }

    pub fn validate_foreign_key_constraints(
        &self,
        row: &OkRow,
        csv_mapping: &CsvMapping,
        schema: &CsvSchema,
    ) -> Vec<ForeignKeyViolation> {
        // println!("validate fk constraints...");

        let mut violations: Vec<ForeignKeyViolation> = Vec::new();

        for fk in &schema.foreign_keys {
            let ref_csv = csv_mapping.get(&fk.references.file).expect("...");
            let ref_schema = &ref_csv.schema;
            let column_selection = ref_schema.named_col2col_selection(&fk.references.columns);

            // create hash from row cells over fk's column_selection
            let row_hash: u64 = row.hash(&column_selection);

            // find hash in fk's referenced csv
            let ref_resolved_fk_row = ref_csv.find_row(&column_selection, row_hash);

            // if no row is found, it means the foreign key constraint is not fulfilled for this row
            if ref_resolved_fk_row.is_err() {
                let fk_violation = ForeignKeyViolation {
                    foreign_key: fk.clone(),
                    line_no: row.line_no,
                };

                violations.push(fk_violation)
            }
        }
        violations
    }

    pub fn validate_row(
        &mut self,
        row: &OkRow,
        schema: &CsvSchema,
        csv_mapping: &CsvMapping,
        // all_rows: &Vec<OkRow>,
        // hashed_rows: &HashMap<ColumnSelection, RowHashMap>,
    ) -> RowCellsValidationResult {
        /*
        validates cells and foreign key constraints for this row
         */

        let cell_violations = self.validate_row_cells(schema, row);

        // let (unique_results, row_) = self.validate_row_unique_constraints(schema, row);

        let fk_violations = self.validate_foreign_key_constraints(row, csv_mapping, schema);

        let has_errors = !cell_violations.is_empty() || !fk_violations.is_empty();

        if has_errors {
            RowCellsValidationResult::Error {
                line_no: row.line_no,
                cell_data_violations: cell_violations,
                fk_violations: fk_violations,
            }
        } else {
            RowCellsValidationResult::Ok {
                line_no: row.line_no,
            }
        }
    }

    pub fn validate_row22(&mut self, row: &OkRow, csv_schema: &CsvSchema) {

        // self.unique_idx_seen.insert(k, v)
    }
}
