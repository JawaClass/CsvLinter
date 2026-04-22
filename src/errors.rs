#[derive(thiserror::Error, Debug)]
pub enum CsvSchemaError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML parse error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("no CSV loaded — call read_csv() before hashing rows")]
    NoCsvLoaded,
}
