use regex::Regex;
use serde::{Deserialize, Deserializer};

pub fn deserialize_regex_opt<'de, D>(deserializer: D) -> Result<Option<Regex>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(deserializer)?;

    let ret = match opt {
        Some(pattern) => Regex::new(&pattern)
            .map(Some)
            .map_err(serde::de::Error::custom),
        None => Ok(None),
    };

    ret
}
