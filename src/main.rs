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
// use crate::csv_cached::CachedCsvHasher;
use crate::csv_workspace::CachedCsvWorkspace;
use crate::regex_serializer::deserialize_regex_opt;

mod csv_cached;
mod csv_cell_validators;
mod csv_reader;
mod csv_row_validators;
mod csv_schema;
mod csv_workspace;
mod util;

// fn write_result(result: Vec<LineEvalResult>, errors_only: bool) {
//     for line in result {
//         let is_error = matches!(
//             line,
//             LineEvalResult::Error { .. } | LineEvalResult::LineSkipped { .. }
//         );

//         if !errors_only || is_error {
//             println!("{:?}", line);
//             println!("- - - - ");
//         }
//     }
// }

// fn build_unique_bitmask(columns: &Vec<CsvColumnSchema>, unique_col_idx: &Vec<usize>) -> Vec<u8> {
//     /*
//     buils a bit mask vector same size as columns length with value 1 where the column is supposed to be unique and 0 otherwise
//      */
//     let bit_mask: Vec<u8> = columns
//         .iter()
//         .enumerate()
//         .map(|(idx, _)| {
//             if unique_col_idx.iter().any(|x| *x == idx) {
//                 1
//             } else {
//                 0
//             }
//         })
//         .collect();

//     bit_mask
// }

// fn build_unique_col_idx(
//     constraint: &SchemaUniqueSpecifier,
//     col_name_2_col_idx: &HashMap<String, usize>,
// ) -> Vec<usize> {
//     /*
//     maps the column names to their index
//      */
//     let unique_col_idx: Vec<usize> = constraint
//         .columns
//         .iter()
//         .map(|col_name| {
//             *col_name_2_col_idx.get(col_name).expect(&format!(
//                 "given unique column name not found in schema: {}",
//                 col_name
//             ))
//         })
//         .collect();

//     unique_col_idx
// }

// fn build_unique_constraint(
//     schema: &CsvSchema,
//     constraint: &SchemaUniqueSpecifier,
// ) -> UniqueConstraint {
//     let col_name_2_col_idx: HashMap<String, usize> = schema
//         .columns
//         .iter()
//         .enumerate() // gives (idx, &column)
//         .map(|(idx, column)| (column.name.clone(), idx))
//         .collect();

//     let unique_col_idx = build_unique_col_idx(constraint, &col_name_2_col_idx);
//     let bit_mask = build_unique_bitmask(&schema.columns, &unique_col_idx);

//     // returns constraint paired with bit_mask
//     UniqueConstraint {
//         constraint: constraint.clone(),
//         bit_mask,
//     }
// }

// fn build_fk_hashmap(rows: Vec<Vec<&str>>, hash_indizes: Vec<usize>) -> HashMap<u64, usize> {
//     /*
//     iterates over all rows, builds a hash based on hash_indizes and returns the hashmap mapping fk hash to row
//      */
//     let mut map: HashMap<u64, usize> = HashMap::new();

//     if rows.is_empty() {
//         return map;
//     }

//     let highest_idx = *hash_indizes.iter().max().expect("Expected non empty vec");

//     assert!(highest_idx < rows.len());

//     // todo get from ok row method
//     // let row_len = rows[0].len();

//     // let mut bitmask = vec![false; row_len];

//     // for i in hash_indizes {
//     //     bitmask[i] = true;
//     // }

//     // for (idx, row) in rows.iter().enumerate() {
//     //     // overwrite duplicate hashes. we only need to know if they exist
//     //     let hash_source: Vec<&str> = row
//     //         .iter()
//     //         .enumerate()
//     //         .filter(|(idx, _)| bitmask[*idx])
//     //         .map(|(_, b)| *b)
//     //         .collect();

//     //     let hash = hashing::hash_vec(&hash_source);
//         map.insert(hash, idx);
//     }

//     map
// }

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut workspace = CachedCsvWorkspace::new();

    workspace.add_csv_file("mt_article.csv", "schema.mt_article.toml");
    workspace.csv("mt_article.csv").load()?;
    workspace.prepare_fk_validation_for("mt_article.csv")?;

    workspace.csv("mt_article.csv").prepare_validation()?;

    workspace.validate_csv("mt_article.csv");

    let validated = &workspace.csv("mt_article.csv").get_validated_rows()?;

    println!("validated rows:{:?}", validated.len());
    println!("{:#?}", validated);

    println!("Program finished.");

    Ok(())
}
