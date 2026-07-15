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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("Hello World"), "hello-world");
        assert_eq!(slugify("My Project!!"), "my-project");
        assert_eq!(slugify("Test---Slug"), "test-slug");
        assert_eq!(slugify("  spaces  "), "spaces");
    }
}
