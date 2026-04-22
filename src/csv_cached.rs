use std::{cell, collections::HashMap, path::Path};

#[derive(thiserror::Error, Debug)]
pub enum CsvCachingError {
    #[error("no CSV loaded")]
    NoCsvLoaded,

    #[error("CSV loaded — but empty.")]
    CsvEmpty,

    #[error("CSV Slice does not exist.")]
    CsvSliceKeyError { columns: ColumnSelection },

    #[error("CSV Row does not exist.")]
    CsvRowKeyError { columns: ColumnSelection, hash: u64 },
}

use crate::{
    csv_reader::{CsvReadError, CsvReadResult, OkRow, build_csv_reader},
    csv_row_validators::{RowValidationResult, validate_row},
    csv_schema::CsvSchema,
};

/// The column indices used as input to a hash function
type ColumnSelection = Vec<usize>;

/// Maps a hash value to the row index in csv.rows
type RowHashMap = HashMap<u64, usize>;

pub struct CachedCsv {
    // filename of csv to read or reread from
    pub filename: String,
    // buffered rows from filename
    csv: Option<CsvReadResult>,
    // indizes (slice) 2  (hash 2 rows idx)
    hashed_rows: HashMap<ColumnSelection, RowHashMap>,
    // the hash function
    hash_func: fn(&[&str]) -> u64,
    // add field for the row eval results...
    pub validated_rows: Vec<RowValidationResult>,
    // schema of csv
    schema: CsvSchema,
}

impl CachedCsv {
    pub fn new(filename: String, schema: CsvSchema, hash_func: fn(&[&str]) -> u64) -> CachedCsv {
        CachedCsv {
            filename: filename,
            hash_func: hash_func,
            hashed_rows: HashMap::new(),
            csv: None,
            validated_rows: vec![],
            schema: schema,
        }
    }

    pub fn csv(&self) -> Result<&CsvReadResult, CsvCachingError> {
        self.csv.as_ref().ok_or(CsvCachingError::NoCsvLoaded)
    }

    pub fn load(&mut self) -> Result<(), CsvReadError> {
        let csv = CachedCsv::read(&self.filename, &self.schema)?;
        self.csv = Some(csv);
        self.clear();
        Ok(())
    }

    pub fn clear(&mut self) {
        self.hashed_rows.clear();
    }

    fn read(filename: &str, schema: &CsvSchema) -> Result<CsvReadResult, CsvReadError> {
        let mut reader = build_csv_reader(filename, &schema.settings)?;
        let csv = CsvReadResult::from_reader(&mut reader, &schema);
        Ok(csv)
    }
    pub fn update_cache(&mut self) -> Result<(), CsvReadError> {
        let is_stale = self.is_cache_stale();
        if !is_stale {
            return Ok(());
        }

        self.load()?;
        Ok(())
    }

    pub fn is_cache_stale(&self) -> bool {
        // check if file has been changed
        let path = Path::new(&self.filename);
        assert!(path.exists() && path.is_file());

        let Some(csv) = &self.csv else {
            return false;
        };

        let modified = path
            .metadata()
            .unwrap()
            .modified()
            .expect("Critical Error: Need the timestamp");

        modified > csv.timestamp
    }

    pub fn clear_validations(&mut self) {
        self.validated_rows.clear();
    }

    pub fn validate_rows(&mut self) {
        self.clear_validations();

        let Some(csv) = &self.csv else {
            println!("Cant validate rows because the csv data is not ok");
            return;
        };

        let rows_to_validate = &csv.ok_rows;

        for row in rows_to_validate {
            let cells = &row.cells;
            let line_no = row.line_no;

            let result = validate_row(row, &self.schema);

            self.validated_rows.push(result);

            println!("CSV Row: [{:?}] {:?}", line_no, cells);
        }
    }
}
pub trait CachedCsvHasher {
    fn index_columns(&mut self, columns: &ColumnSelection) -> Result<(), CsvCachingError>;
    fn find_row(&self, columns: &ColumnSelection, hash: u64) -> Result<&OkRow, CsvCachingError>;
}

impl CachedCsvHasher for CachedCsv {
    fn index_columns(&mut self, columns: &ColumnSelection) -> Result<(), CsvCachingError> {
        let Some(csv) = &self.csv else {
            return Err(CsvCachingError::NoCsvLoaded);
        };

        let rows = &csv.ok_rows;

        if rows.is_empty() {
            return Err(CsvCachingError::CsvEmpty);
        }

        let slice_idx = columns.to_vec();

        // gets existing column selection hashmap or create new empty one
        let slice = self.hashed_rows.entry(slice_idx).or_default();

        let row_len = rows[0].cells.len();

        let mut bitmask = vec![false; row_len];

        for i in columns {
            bitmask[*i] = true;
        }

        for (idx, row) in rows.iter().enumerate() {
            let row_hash_source: Vec<&str> = row
                .cells
                .iter()
                .enumerate()
                .filter(|(idx, _)| bitmask[*idx])
                .map(|(_, b)| b.as_str())
                .collect();

            let hash = (self.hash_func)(&row_hash_source);
            slice.insert(hash, idx);
        }

        Ok(())
    }
    // }

    fn find_row(&self, columns: &ColumnSelection, hash: u64) -> Result<&OkRow, CsvCachingError> {
        let Some(csv) = &self.csv else {
            return Err(CsvCachingError::NoCsvLoaded);
        };

        let rows = &csv.ok_rows;

        // get slice or return error
        let slice = self
            .hashed_rows
            .get(columns)
            .ok_or(CsvCachingError::CsvSliceKeyError {
                columns: columns.clone(),
            })?;

        // get hash from slice or return error
        let row_idx = slice.get(&hash).ok_or(CsvCachingError::CsvRowKeyError {
            columns: columns.clone(),
            hash,
        })?;

        let ret = &rows[*row_idx];

        Ok(ret)
    }
}
