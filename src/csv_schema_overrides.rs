use std::collections::HashMap;

use crate::{
    csv_reader::OkRow,
    csv_schema::{Conditional, CsvColumnSchema, CsvSchema},
};

pub fn eval_column_type_for_cell(
    row_map: &HashMap<String, String>,
    column_idx: usize,
    schema_columns: &Vec<CsvColumnSchema>,
) -> CsvColumnSchema {
    let coll_schema = schema_columns[column_idx].clone();

    let required_default = coll_schema.required.or(None);

    match &coll_schema.overrides {
        // if no overrides, simply return the column schema
        None => coll_schema.clone(),
        // if overrides, iterate over them and return the first which conditional yields positive
        Some(overrides) => {
            for ovride in overrides {
                let result = ovride.condition.evaluate(&row_map);

                if result {
                    return CsvColumnSchema {
                        dtype: ovride.dtype.clone(),
                        name: coll_schema.name.clone(),
                        overrides: None,
                        required: ovride.required.or(required_default),
                    };
                }
            }

            coll_schema
        }
    }
}
