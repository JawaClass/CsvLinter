#[macro_export]
macro_rules! input {
    ($prompt:expr, $typ:ty) => {{
        let mut input = String::new();
        print!("{}", $prompt);
        io::stdout().flush().expect("Failed to flush stdout");
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");
        input.trim().parse::<$typ>().expect("Failed to parse input")
    }};

    // Pattern without type → returns String
    ($prompt:expr) => {{
        let mut input = String::new();
        print!("{}", $prompt);
        io::stdout().flush().expect("Failed to flush stdout");
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");
        input.trim().to_string()
    }};
}
