use socs_backend::utils::sanitize::{sanitize_html, sanitize_url};

#[test]
fn rich_text_removes_scripts_handlers_and_unsafe_links() {
    let sanitized = sanitize_html(
        r#"<p onclick="alert(1)">Hello</p><script>alert(1)</script><a href="javascript:alert(1)">bad</a>"#,
    );

    assert!(sanitized.contains("<p>Hello</p>"));
    assert!(!sanitized.contains("<script"));
    assert!(!sanitized.contains("onclick"));
    assert!(!sanitized.contains("javascript:"));
}

#[test]
fn stored_url_fields_only_accept_http_schemes() {
    assert_eq!(sanitize_url("https://example.com/image.png"), Some("https://example.com/image.png".to_string()));
    assert_eq!(sanitize_url("javascript:alert(1)"), None);
    assert_eq!(sanitize_url("data:text/html,boom"), None);
    assert_eq!(sanitize_url("/relative/path"), None);
}
