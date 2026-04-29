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
use crate::csv_cached::RowValidationResult;
use crate::csv_row_validators::RowCellsValidationResult;
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

// fn write_result(result: &Vec<RowValidationResult>, errors_only: bool) {
//     println!("###############################");
//     println!("WRITE CSV ROW VALIDATION RESULTS");
//     println!(" - {:?} x", result.len());
//     println!("###############################");
//     let mut skipped_ok_lines = 0;
//     let mut print_idx = 0;
//     let mut ok_lines = 0;

//     for line in result {
//         let is_ok = line.unique_violations.is_empty()
//             && matches!(line.row_result, RowCellsValidationResult::Ok { .. });

//         if is_ok {
//             ok_lines += 1;
//         }
//         if errors_only && is_ok {
//             skipped_ok_lines += 1;
//             continue;
//         }

//         print_idx += 1;
//         println!("Row Validtion Result... {}", print_idx);
//         println!("{:?}", line);
//         println!("- - - - ");
//     }
//     println!("OK lines not printed: {:?}", skipped_ok_lines);
//     println!(
//         "lines ok {} / {} ({}% OK)",
//         ok_lines,
//         result.len(),
//         (ok_lines / result.len()) * 100
//     );
fn write_result(result: &Vec<RowValidationResult>, errors_only: bool) {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📋 CSV ROW VALIDATION RESULTS");
    println!("   {} rows total", result.len());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let mut skipped_ok_lines = 0;
    let mut print_idx = 0;
    let mut ok_lines = 0;

    for line in result {
        let is_ok = line.unique_violations.is_empty()
            && matches!(line.row_result, RowCellsValidationResult::Ok { .. });
        if is_ok {
            ok_lines += 1;
        }
        if errors_only && is_ok {
            skipped_ok_lines += 1;
            continue;
        }
        print_idx += 1;
        println!("─────────────────────────────── Row {}", print_idx);
        println!("{:?}", line);
    }

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    if skipped_ok_lines > 0 {
        println!("🙈 {} OK rows hidden (errors_only mode)", skipped_ok_lines);
    }
    let pct = if result.len() > 0 {
        (ok_lines * 100) / result.len()
    } else {
        0
    };
    let icon = if pct == 100 {
        "🎉"
    } else if pct >= 80 {
        "✅"
    } else {
        "❌"
    };
    println!("{} {}/{} rows OK ({}%)", icon, ok_lines, result.len(), pct);
}
// }

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut workspace = CachedCsvWorkspace::new();

    let mt_article = "mt_article.csv";

    let mt_type = "mt_type.csv";

    workspace.run_validation_steps(mt_article)?;
    let validated = &workspace.csv(mt_article).get_validated_rows()?;
    write_result(validated, true);


    workspace.run_validation_steps(mt_type)?;

    let validated = &workspace.csv(mt_type).get_validated_rows()?;
    write_result(validated, true);



    println!("Program finished.");
    Ok(())
}
