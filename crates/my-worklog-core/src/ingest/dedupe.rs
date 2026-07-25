pub fn content_hash(input: &str) -> String {
    blake3::hash(input.as_bytes()).to_hex().to_string()
}
