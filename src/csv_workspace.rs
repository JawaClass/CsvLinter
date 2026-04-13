use std::{collections::HashMap, path::Path};

use crate::{csv_cached::CachedCsv, hashing};

use crate::{csv_reader::build_csv_reader, csv_reader::read_csv};

use crate::csv_schema::read_csv_schema;

pub struct CachedCsvWorkspace {
    // filename 2 cache csv
    mapping: HashMap<String, CachedCsv>,
}

impl CachedCsvWorkspace {
    pub fn new() -> CachedCsvWorkspace {
        let mapping: HashMap<String, CachedCsv> = HashMap::new();
        CachedCsvWorkspace { mapping }
    }

    pub fn files(&self) -> Vec<&str> {
        self.mapping.keys().map(|k| k.as_str()).collect()
    }

    pub fn csv_or_panic(&self, filename: &str) -> &CachedCsv {
        self.mapping
            .get(filename)
            .expect("filename does not exist in workspace")
    }

    pub fn add_csv_file(&mut self, csv: &str, schema: &str) {
        if self.mapping.contains_key(csv) {
            panic!("csv already loaded: {}", csv);
        }

        let csv_path = Path::new(&csv);
        let schema_path = Path::new(&schema);

        assert!(
            csv_path.exists() && csv_path.is_file(),
            "csv file not a file {}",
            csv
        );

        assert!(
            schema_path.exists() && schema_path.is_file(),
            "schema file not a file {}",
            schema
        );

        let schema = read_csv_schema(&schema_path).expect("Cant read CSV Schema");

        let mut csv_reader = build_csv_reader(&csv, &schema.settings);
        let csv_parsed = read_csv(&mut csv_reader, &schema);

        self.mapping.insert(
            csv.to_string(),
            CachedCsv::new(csv.to_string(), csv_parsed, hashing::hash_vec),
        );
    }
}
