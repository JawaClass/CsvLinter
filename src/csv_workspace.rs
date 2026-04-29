use std::fmt::format;
use std::{collections::HashMap, path::Path};

use csv::Error;

use crate::csv_cached::{CsvCachingError, IndexingDuplicateInfo};
use crate::{csv_cached::CachedCsv, hashing};

use crate::{csv_reader::build_csv_reader, csv_reader::read_csv};

use crate::csv_schema::read_csv_schema;
pub type CsvMapping = HashMap<String, CachedCsv>;
pub struct CachedCsvWorkspace {
    // filename 2 cache csv
    mapping: CsvMapping,
}

impl CachedCsvWorkspace {
    pub fn new() -> CachedCsvWorkspace {
        let mapping: HashMap<String, CachedCsv> = HashMap::new();
        CachedCsvWorkspace { mapping }
    }

    pub fn clear(&mut self) {
        self.mapping.clear();
    }

    pub fn remove(&mut self, filename: &str) {
        self.mapping.remove(filename);
    }

    pub fn validate_csv(&mut self, filename: &str) {
        let mut csv = self.mapping.remove(filename).expect("...");
        csv.validate_rows(&self.mapping);
        self.mapping.insert(filename.to_string(), csv);
    }

    pub fn files(&self) -> Vec<&str> {
        self.mapping.keys().map(|k| k.as_str()).collect()
    }

    pub fn has_file(&self, filename: &str) -> bool {
        self.mapping.contains_key(filename)
    }

    pub fn csv(&mut self, filename: &str) -> &mut CachedCsv {
        self.mapping
            .get_mut(filename)
            .expect("filename does not exist in workspace")
    }

    pub fn prepare_fk_validation_for(&mut self, filename: &str) -> Result<(), CsvCachingError> {
        // let rows = &self.csv(filename).content()?.ok_rows;
        let schema = &self.csv(filename).schema.clone();

        println!("prepare fk validation for... {}", filename);

        for fk in schema.foreign_keys.iter() {
            println!("FK: {:?}", fk);

            let filename = &fk.references.file;
            if !self.has_file(filename) {
                let fk_schema = format!("schema.{}", filename).replace(".csv", ".toml");
                println!(
                    "fk referenced file not loaded. {:?} --- {:?}",
                    filename, fk_schema
                );
                self.add_csv_file(filename, &fk_schema);
                self.csv(filename).load()?;
            }

            fn ignore_duplicates(_: &IndexingDuplicateInfo) {}

            let columns = schema.named_col2col_selection(&fk.references.columns);
            self.csv(filename)
                .index_columns(&columns, ignore_duplicates)?;
        }

        Ok(())
    }
    pub fn run_validation_steps(&mut self, csv_filename: &str) -> Result<(), CsvCachingError> {
        let schema_filename = format!("schema.{}.toml", csv_filename.trim_end_matches(".csv"));
        self.remove(csv_filename);
        self.add_csv_file(csv_filename, &schema_filename);
        self.csv(csv_filename).load()?;
        self.prepare_fk_validation_for(csv_filename)?;
        self.csv(csv_filename).prepare_validation()?;
        self.validate_csv(csv_filename);
        Ok(())
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

        // let mut csv_reader = build_csv_reader(&csv, &schema.settings);
        // let csv_parsed = read_csv(&mut csv_reader, &schema);

        self.mapping.insert(
            csv.to_string(),
            CachedCsv::new(csv.to_string(), schema, hashing::hash_vec),
        );
    }
}
