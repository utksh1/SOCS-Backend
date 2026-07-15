use socs_backend::utils::sanitize::*;

// --- Migrated from inline tests ---

#[test]
fn test_sanitize_html_removes_script_tags() {
    let malicious = r#"<p>Hello</p><script>alert('XSS')</script>"#;
    let sanitized = sanitize_html(malicious);
    assert!(!sanitized.contains("<script"));
    assert!(sanitized.contains("<p>Hello</p>"));
}

#[test]
fn test_sanitize_html_removes_event_handlers() {
    let malicious = r#"<div onclick="alert('XSS')">Click me</div>"#;
    let sanitized = sanitize_html(malicious);
    assert!(!sanitized.contains("onclick"));
}

#[test]
fn test_sanitize_html_preserves_safe_content() {
    let safe = r#"<p>This is <strong>bold</strong> and <em>italic</em></p>"#;
    let sanitized = sanitize_html(safe);
    assert!(sanitized.contains("<strong>"));
    assert!(sanitized.contains("<em>"));
}

#[test]
fn test_sanitize_url_blocks_javascript() {
    assert_eq!(sanitize_url("javascript:alert('XSS')"), None);
    assert_eq!(sanitize_url("JAVASCRIPT:alert('XSS')"), None);
}

#[test]
fn test_sanitize_url_blocks_data_urls() {
    assert_eq!(sanitize_url("data:text/html,<script>alert('XSS')</script>"), None);
}

#[test]
fn test_sanitize_url_allows_safe_urls() {
    let url = "https://example.com/page";
    assert_eq!(sanitize_url(url), Some(url.to_string()));
}

#[test]
fn test_strip_html_tags() {
    let html = r#"<p>Hello <strong>World</strong></p>"#;
    let stripped = strip_html_tags(html);
    assert!(stripped.contains("Hello"));
    assert!(stripped.contains("World"));
    assert!(!stripped.contains("<"));
    assert!(!stripped.contains(">"));
}

#[test]
fn test_sanitize_plain_text() {
    let text = "<script>alert('XSS')</script>";
    let sanitized = sanitize_plain_text(text);
    assert!(sanitized.contains("&lt;script&gt;"));
}

// --- New expanded tests ---

#[test]
fn test_sanitize_url_blocks_vbscript() {
    assert_eq!(sanitize_url("vbscript:msgbox('XSS')"), None);
    assert_eq!(sanitize_url("VBSCRIPT:msgbox('XSS')"), None);
}

#[test]
fn test_sanitize_url_blocks_file_protocol() {
    assert_eq!(sanitize_url("file:///etc/passwd"), None);
    assert_eq!(sanitize_url("FILE:///etc/passwd"), None);
}

#[test]
fn test_sanitize_html_handles_nested_attacks() {
    let nested = r#"<div><script>alert('XSS')</script><p>Safe content</p></div>"#;
    let sanitized = sanitize_html(nested);
    assert!(!sanitized.contains("<script"));
    assert!(sanitized.contains("<p>Safe content</p>"));
}

#[test]
fn test_sanitize_url_handles_mixed_case_protocols() {
    assert_eq!(sanitize_url("JaVaScRiPt:alert('XSS')"), None);
    assert_eq!(sanitize_url("DaTa:text/html,<script>"), None);
}

#[test]
fn test_sanitize_html_preserves_safe_tables() {
    let table = r#"<table><tr><th>Header</th></tr><tr><td>Data</td></tr></table>"#;
    let sanitized = sanitize_html(table);
    assert!(sanitized.contains("<table>"));
    assert!(sanitized.contains("<tr>"));
    assert!(sanitized.contains("<th>"));
    assert!(sanitized.contains("<td>"));
}
