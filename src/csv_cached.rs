use std::{collections::HashMap, path::Path};

use crate::{
    csv_reader::{CsvReadResult, OkRow, build_csv_reader},
    csv_row_validators::RowValidationResult,
    csv_schema::CsvSchema,
};

pub struct CachedCsv {
    pub filename: String,
    // buffered rows from filename
    pub csv: Option<CsvReadResult>,
    // indizes (slice) 2  (hash 2 rows idx)
    hashed_rows: HashMap<Vec<usize>, HashMap<u64, usize>>, // TODO: change to vec of usize to allow multiple entries per hash in file
    // the hash function
    hash_func: fn(&[&str]) -> u64,

    // add field for the row eval results...
    validated_rows: Vec<RowValidationResult>,

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

    pub fn load(&mut self) {
        let csv = CachedCsv::read(&self.filename, &self.schema);

        self.csv = Some(csv);
    }

    fn read(filename: &str, schema: &CsvSchema) -> CsvReadResult {
        let mut reader = build_csv_reader(filename, &schema.settings);
        let csv = CsvReadResult::from_reader(&mut reader, &schema);
        csv
    }
    pub fn update_cache(&mut self) {
        let is_stale = self.is_cache_stale();
        if !is_stale {
            return;
        }

        self.load();
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
        let Some(csv) = &self.csv else {
            return;
        };
        let rows_to_validate = &csv.ok_rows;

        for row in rows_to_validate {
            let cells = &row.cells;
            let line_no = row.line_no;
        }
    }
}
pub trait CachedCsvHasher {
    fn hash_rows_for(&mut self, slice_indizes: &[usize]);
    fn hashed_row_in(&self, slice_indizes: &[usize], hash: u64) -> &OkRow;
}

impl CachedCsvHasher for CachedCsv {
    fn hash_rows_for(&mut self, indizes: &[usize]) {
        let rows = &self.csv.ok_rows;
        println!(
            "hash rows for indizes. {:?}. rowLen: {:?}",
            indizes,
            rows.len()
        );
        if rows.is_empty() {
            return;
        }

        let slice_idx = indizes.to_vec();
        let slice = self.hashed_rows.entry(slice_idx).or_default();

        let row_len = rows[0].cells.len();

        let mut bitmask = vec![false; row_len];

        for i in indizes {
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
    }

    fn hashed_row_in(&self, slice_indices: &[usize], hash: u64) -> &OkRow {
        let rows = &self.csv.ok_rows;

        let slice = self
            .hashed_rows
            .get(slice_indices)
            .expect("hash for slice does not exist");

        let row_idx = slice.get(&hash).expect("row for hash does not exist");

        let ret = &rows[*row_idx];

        ret
    }
}
