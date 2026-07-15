/// Convert a string into a URL-friendly slug
/// 
/// Examples:
/// - "Hello World" -> "hello-world"
/// - "My Project!!" -> "my-project"
/// - "Test---Slug" -> "test-slug"
pub fn slugify(s: &str) -> String {
    s.to_lowercase()
        .trim()
        .replace(|c: char| !c.is_alphanumeric() && c != '-', "-")
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}


