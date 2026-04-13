use rhai::{Dynamic, Engine, Map};
use std::cell::LazyCell;
use std::collections::HashMap;

const RHAI_ENGINE: LazyCell<Engine> = LazyCell::new(Engine::new);

pub fn run_custom_validator(
    script: &str,
    value: &str,
    row: &HashMap<String, String>,
) -> Result<bool, Box<rhai::EvalAltResult>> {

    // Build row map accessible in script as `row["col"]`
    let mut rhai_row = Map::new();
    for (k, v) in row {
        rhai_row.insert(k.clone().into(), Dynamic::from(v.clone()));
    }

    let mut scope = rhai::Scope::new();
    scope.push("value", value.to_string());
    scope.push("row", rhai_row);

    println!("eval script with rhai: value={}", value);

    RHAI_ENGINE.eval_with_scope::<bool>(&mut scope, script)
}
