use std::{cell, collections::HashMap, hash::Hash, path::Path, vec};

#[derive(thiserror::Error, Debug)]
pub enum CsvCachingError {
    #[error("no CSV loaded")]
    NoCsvLoaded,

    #[error("CSV loaded — but empty.")]
    CsvEmpty,

    #[error("CSV Slice does not exist.")]
    CsvSliceKeyError {
        columns: ColumnSelection,
        available_columns: Vec<ColumnSelection>,
    },

    #[error("CSV Row does not exist.")]
    CsvRowKeyError { columns: ColumnSelection, hash: u64 },

    #[error("CSV Row does not exist.")]
    CsvRowIndexError { index: usize },

    #[error("Unique Constraint Result for this Row does not exist.")]
    CsvRowUniqueConstraintIndexError { index: usize },

    #[error("Cant load csv.")]
    CsvReadError(CsvReadError),
}

impl From<CsvReadError> for CsvCachingError {
    fn from(e: CsvReadError) -> Self {
        CsvCachingError::CsvReadError(e)
    }
}

use crate::{
    csv_reader::{CsvReadError, CsvReadResult, OkRow, build_csv_reader},
    csv_row_validators::{
        ColumnSelection, CsvValidator, ForeignKeyViolation, RowCellsValidationResult, RowHashMap,
        UniqueConstraint, UniqueViolation,
    },
    csv_schema::CsvSchema,
    csv_workspace::{CachedCsvWorkspace, CsvMapping},
};

pub struct CachedCsv {
    // filename of csv to read or reread from
    pub filename: String,
    // buffered rows from filename
    csv: Option<CsvReadResult>,
    // indizes (slice) 2  (hash 2 rows idx)
    hashed_rows: HashMap<ColumnSelection, RowHashMap>,
    // the hash function
    hash_func: fn(&[&str]) -> u64,
    // row idx 2 validation result
    validated_rows: HashMap<usize, RowCellsValidationResult>,
    // row idx 2 unqie volations
    validated_unique_constraints: HashMap<usize, Vec<UniqueViolation>>,
    // row idx 2 foreign key volations
    validated_foreign_key_constraints: HashMap<usize, Vec<ForeignKeyViolation>>,
    // schema of csv
    pub schema: CsvSchema,
}

pub struct IndexingDuplicateInfo {
    pub new_element_idx: usize,
    pub existing_element_idx: usize,
}

impl CachedCsv {
    pub fn new(filename: String, schema: CsvSchema, hash_func: fn(&[&str]) -> u64) -> CachedCsv {
        CachedCsv {
            filename: filename,
            hash_func: hash_func,
            hashed_rows: HashMap::new(),
            csv: None,
            validated_rows: HashMap::new(),
            validated_unique_constraints: HashMap::new(),
            validated_foreign_key_constraints: HashMap::new(),
            schema: schema,
        }
    }

    pub fn get_validated_rows(
        &self,
    ) -> Result<Vec<(&RowCellsValidationResult, Option<&Vec<UniqueViolation>>)>, CsvCachingError>
    {
        let csv = self.content()?;

        let rows = &csv.ok_rows;

        let validated_rows: Vec<(&RowCellsValidationResult, Option<&Vec<UniqueViolation>>)> = rows
            .iter()
            .enumerate()
            .map(|(row_idx, row)| self.get_validated_row(row_idx))
            // .collect();
            .collect::<Result<Vec<_>, _>>()?;

        Ok(validated_rows)
    }

    pub fn get_validated_row(
        &self,
        idx: usize,
    ) -> Result<(&RowCellsValidationResult, Option<&Vec<UniqueViolation>>), CsvCachingError> {
        /*
        collects all internal validation results for this row index ans returns it combined
         */
        let row_result = &self
            .validated_rows
            .get(&idx)
            .ok_or(CsvCachingError::CsvRowIndexError { index: idx })?;
        let unique_result = self.validated_unique_constraints.get(&idx);

        Ok((row_result, unique_result))
    }

    pub fn content(&self) -> Result<&CsvReadResult, CsvCachingError> {
        self.csv.as_ref().ok_or(CsvCachingError::NoCsvLoaded)
    }

    pub fn load(&mut self) -> Result<(), CsvReadError> {
        /*
        reads the csv file content into memory
         */
        let csv = CachedCsv::read(&self.filename, &self.schema)?;
        self.csv = Some(csv);
        self.clear();
        Ok(())
    }

    pub fn clear(&mut self) {
        self.hashed_rows.clear();
        self.validated_rows.clear();
        self.validated_foreign_key_constraints.clear();
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

    fn clear_validations(&mut self) {
        self.validated_rows.clear();
        self.validated_unique_constraints.clear();
    }

    pub fn validate_rows(&mut self, csv_mapping: &CsvMapping) {
        let mut validator = CsvValidator::new(&self.schema);

        let Some(csv) = &self.csv else {
            println!("Cant validate rows because the csv data is not ok");
            return;
        };

        let rows = &csv.ok_rows;

        for (row_idx, row) in rows.iter().enumerate() {
            let result = validator.validate_row(row, &self.schema, &csv_mapping);
            self.validated_rows.insert(row_idx, result);
        }
    }

    pub fn prepare_validation(&mut self) -> Result<(), CsvCachingError> {
        self.clear_validations();
        // index unique constraints
        self.index_unique_constraints()?;
        // fk contraints are managed via workspace...

        Ok(())
    }

    pub fn index_unique_constraints(&mut self) -> Result<(), CsvCachingError> {
        // Precompute everything that depends on schema

        println!("index unique constraints...");

        let constraints: Vec<(ColumnSelection, String)> = {
            let unique_constraints = &self.schema.unique;
            unique_constraints
                .iter()
                .map(|constraint| {
                    (
                        self.schema.named_col2col_selection(&constraint.columns),
                        constraint.name.clone(),
                    )
                })
                .collect()
        };

        // Only store indices for now (no borrowing of rows!)
        let mut collected: Vec<(usize, usize, usize)> = Vec::new();
        // (new_idx, existing_idx, constraint_idx)

        for (constraint_idx, (column_selection, _)) in constraints.iter().enumerate() {
            let mut on_duplicate = |dup: &IndexingDuplicateInfo| {
                collected.push((
                    dup.new_element_idx,
                    dup.existing_element_idx,
                    constraint_idx,
                ));
            };

            self.index_columns(&column_selection, &mut on_duplicate)?;
        }

        for (new_idx, existing_idx, constraint_idx) in collected {
            let other_line_no = {
                let rows = &self.content()?.ok_rows;
                rows[existing_idx].line_no
            }; // 

            self.validated_unique_constraints
                .entry(new_idx)
                .or_default()
                .push(UniqueViolation {
                    line_no: other_line_no,
                    constraint_name: constraints[constraint_idx].1.clone(),
                });
        }

        Ok(())
    }

    // pub fn index_columns(
    //     &mut self,
    //     columns: &ColumnSelection,
    //     on_index_duplicate: FnMut(duplicate_info: &IndexingDuplicateInfo) -> (),
    // ) -> Result<(), CsvCachingError> {
    pub fn index_columns<F>(
        &mut self,
        columns: &ColumnSelection,
        mut on_index_duplicate: F,
    ) -> Result<(), CsvCachingError>
    where
        F: FnMut(&IndexingDuplicateInfo), // FnMut because your closure mutates self
    {
        println!("index_columns: {:?} . {:?}", self.filename, columns);

        let rows = match &self.csv {
            Some(csv) => &csv.ok_rows,
            None => return Err(CsvCachingError::NoCsvLoaded),
        };

        // create index slice even if rows are empty for more detailed error handling
        let slice_idx = columns.to_vec();

        // gets existing column selection hashmap or create new empty one
        let slice = self.hashed_rows.entry(slice_idx).or_default();

        if rows.is_empty() {
            return Err(CsvCachingError::CsvEmpty);
        }

        let row_len = rows[0].cells.len();

        let mut bitmask = vec![false; row_len];

        for i in columns {
            bitmask[*i] = true;
        }

        for (row_idx, row) in rows.iter().enumerate() {
            let row_hash_source: Vec<&str> = row
                .cells
                .iter()
                .enumerate()
                .filter(|(idx, _)| bitmask[*idx])
                .map(|(_, b)| b.as_str())
                .collect();

            let hash = (self.hash_func)(&row_hash_source);

            if slice.contains_key(&hash) {
                let violates_with_row_idx = slice
                    .get(&hash)
                    .expect("This should exist! Forogot contains key check?");
                // let violates_with_row = &rows[*violates_with_row_idx];

                on_index_duplicate(&IndexingDuplicateInfo {
                    new_element_idx: row_idx,
                    existing_element_idx: *violates_with_row_idx,
                });
                // panic!("INDEX DUPLICATE AT INDEX_COLUMNS DETECTED {:?}", hash);
            }

            println!(" - insert hash for row. {:?} ;; {:?}", row, columns);
            slice.insert(hash, row_idx);
        }

        Ok(())
    }
    // }

    pub fn find_row(
        &self,
        columns: &ColumnSelection,
        hash: u64,
    ) -> Result<&OkRow, CsvCachingError> {
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
                available_columns: self
                    .hashed_rows
                    .keys()
                    .map(|c| c.clone())
                    .collect::<Vec<ColumnSelection>>(),
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
