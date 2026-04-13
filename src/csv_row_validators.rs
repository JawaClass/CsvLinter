use crate::{CellEvalResult, csv_schema::SchemaUniqueSpecifier};

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
        cell_eval_results: Vec<CellEvalResult>,
    },
    Skipped {
        line_no: u64,
        reason: LineSkippedReason,
    },
    Error {
        line_no: u64,
        cell_eval_results: Vec<CellEvalResult>,
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
