use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub fn hash_vec(strings: &[&str]) -> u64 {
    let mut hasher = DefaultHasher::new();
    strings.hash(&mut hasher);
    hasher.finish()
}