use std::{cell, collections::HashMap, vec};

use crate::{
    csv_cell_validators::{CellValidationError, ValueValidationResult},
    csv_reader::OkRow,
    csv_schema::{CellEvaluator, CsvSchema, SchemaUniqueSpecifier},
};

#[derive(Debug)]
enum CellValidationResult {
    Ok {
        value: String,
        col_no: u16,
        line_no: u64,
    },
    Error {
        value: String,
        col_no: u16,
        line_no: u64,
        errors: Vec<CellValidationError>,
    },
}

#[derive(Clone, Debug)]
pub struct UniqueConstraint {
    constraint: SchemaUniqueSpecifier,
    bit_mask: Vec<u8>,
}

#[derive(Debug)]
pub struct UniqueViolation {
    line_no: u64,
    constraint: UniqueConstraint,
}

#[derive(Debug)]
pub enum LineSkippedReason {
    ColumnMismatch { expected: u64, actual: u64 },
    UnknownError {},
}
#[derive(Debug)]
pub enum RowValidationResult {
    Ok {
        line_no: u64,
        cell_validation_results: Vec<CellValidationResult>,
    },
    Skipped {
        line_no: u64,
        reason: LineSkippedReason,
    },
    Error {
        line_no: u64,
        cell_validation_results: Vec<CellValidationResult>,
        unique_violations: Vec<UniqueViolation>,
    },
}
// trait evaluate value

impl RowValidationResult {
    fn line_no(&self) -> u64 {
        match self {
            Self::Ok { line_no, .. }
            | Self::Skipped { line_no, .. }
            | Self::Error { line_no, .. } => *line_no,
        }
    }
}

fn validate_row_unique_constraint(row: &OkRow, constraint: UniqueConstraint) {

        let col_indizes = schema.eval_indizes_unique_constraint(&constraint);
}

pub fn validate_row(row: &OkRow, schema: &CsvSchema) -> RowValidationResult {
    // build mapping column name to value
    // required to pass to rhai for custom cell validation
    // TODO: make lazy so it only gets created when rhai gets called...

    let row_map: HashMap<String, String> = schema
        .columns
        .iter()
        .enumerate()
        .map(|(idx, col)| (col.name.clone(), row.cells[idx].clone()))
        .collect();

    let mut row_has_error = false;

    let mut cell_validation_results = Vec::new();
    for (col_idx, column_schema) in schema.columns.iter().enumerate() {
        let cell_value = &row.cells[col_idx];

        let result = column_schema.dtype.eval_value(cell_value, &row_map);
        // schema.unique[0].columns

        if let ValueValidationResult::Error { value, errors } = &result {
            row_has_error = true;
            let result = CellValidationResult::Error {
                value: value.to_string(),
                col_no: col_idx as u16,
                line_no: row.line_no,
                errors: errors.clone(),
            };
            cell_validation_results.push(result);
        }

        if let ValueValidationResult::Ok { value } = &result {
            let result = CellValidationResult::Ok {
                value: value.to_string(),
                col_no: col_idx as u16,
                line_no: row.line_no,
            };
            cell_validation_results.push(result);
        }
    }

    for constraint in &schema.unique {





    }

    if row_has_error {
        return RowValidationResult::Error {
            line_no: row.line_no,
            cell_validation_results,
            unique_violations: vec![],
        };
    }

    RowValidationResult::Ok {
        line_no: row.line_no,
        cell_validation_results,
    }
}
