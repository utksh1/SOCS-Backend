use socs_backend::utils::slugify::slugify;

// --- Migrated from inline tests ---

#[test]
fn test_slugify_basic_conversion() {
    assert_eq!(slugify("Hello World"), "hello-world");
}

#[test]
fn test_slugify_special_characters() {
    assert_eq!(slugify("My Project!!"), "my-project");
}

#[test]
fn test_slugify_multiple_dashes() {
    assert_eq!(slugify("Test---Slug"), "test-slug");
}

#[test]
fn test_slugify_whitespace_trimming() {
    assert_eq!(slugify("  spaces  "), "spaces");
}

// --- New expanded tests ---

#[test]
fn test_slugify_unicode_characters() {
    // Unicode non-alphanumeric chars get replaced with dashes, then collapsed
    let result = slugify("Café París");
    assert!(!result.contains(' '));
    // The é and í are non-ASCII, slugify replaces them with dashes
    // Exact output depends on implementation but should be lowercase and dash-separated
    assert_eq!(result, result.to_lowercase());
}

#[test]
fn test_slugify_very_long_strings() {
    let long_input = "a".repeat(1000);
    let result = slugify(&long_input);
    assert_eq!(result, "a".repeat(1000));
    assert_eq!(result.len(), 1000);
}

#[test]
fn test_slugify_empty_after_filtering() {
    assert_eq!(slugify("!!!???"), "");
    assert_eq!(slugify("---"), "");
    assert_eq!(slugify("   "), "");
}

#[test]
fn test_slugify_with_numbers() {
    assert_eq!(slugify("Project 2024"), "project-2024");
    assert_eq!(slugify("v1.2.3"), "v1-2-3");
}
