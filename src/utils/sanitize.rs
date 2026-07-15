use ammonia::Builder;
use std::collections::HashSet;

/// Sanitize HTML content to prevent XSS attacks
/// This function removes potentially dangerous HTML elements and attributes
/// while preserving safe formatting tags for rich text content
pub fn sanitize_html(content: &str) -> String {
    // Create a custom ammonia builder with allowed tags and attributes
    let mut builder = Builder::default();
    
    // Define allowed tags for rich text content
    let allowed_tags: HashSet<&str> = [
        // Text formatting
        "p", "br", "strong", "em", "u", "s", "sub", "sup",
        // Headings
        "h1", "h2", "h3", "h4", "h5", "h6",
        // Lists
        "ul", "ol", "li",
        // Links (with restrictions)
        "a",
        // Blockquote and code
        "blockquote", "code", "pre",
        // Tables
        "table", "thead", "tbody", "tr", "th", "td",
        // Divs and spans (for styling)
        "div", "span",
        // Images (with restrictions)
        "img",
        // Horizontal rule
        "hr",
    ].iter().cloned().collect();
    
    builder.tags(allowed_tags);
    
    // Define allowed attributes per tag
    builder
        .link_rel(Some("noopener noreferrer")) // Security: prevent tabnabbing
        .url_relative(ammonia::UrlRelative::Deny) // Only allow absolute URLs
        .add_tag_attributes("a", &["href", "title", "target"])
        .add_tag_attributes("img", &["src", "alt", "title", "width", "height"])
        .add_tag_attributes("div", &["class"])
        .add_tag_attributes("span", &["class"])
        .add_tag_attributes("code", &["class"]) // For syntax highlighting
        .add_tag_attributes("pre", &["class"])
        .add_tag_attributes("table", &["class"])
        .add_tag_attributes("td", &["colspan", "rowspan"])
        .add_tag_attributes("th", &["colspan", "rowspan"]);
    
    // Only allow safe URL schemes
    builder.url_schemes(["http", "https", "mailto"].iter().cloned().collect());
    
    // Clean the HTML
    builder.clean(content).to_string()
}

/// Sanitize plain text by escaping HTML entities
/// Use this for fields that should not contain any HTML
pub fn sanitize_plain_text(text: &str) -> String {
    html_escape::encode_text(text).to_string()
}

/// Strip all HTML tags from content, leaving only text
/// Useful for excerpts or preview text
pub fn strip_html_tags(content: &str) -> String {
    // Use ammonia with empty tag set to strip all HTML
    Builder::default()
        .tags(HashSet::new())
        .clean(content)
        .to_string()
}

/// Validate and sanitize URL inputs
pub fn sanitize_url(url: &str) -> Option<String> {
    // Basic URL validation
    if url.trim().is_empty() {
        return None;
    }
    
    // Check for javascript: and data: schemes (XSS vectors)
    let lower_url = url.trim().to_lowercase();
    if lower_url.starts_with("javascript:") || 
       lower_url.starts_with("data:") || 
       lower_url.starts_with("vbscript:") ||
       lower_url.starts_with("file:") {
        return None;
    }
    
    // Ensure URL starts with http:// or https://
    if lower_url.starts_with("http://") || lower_url.starts_with("https://") {
        Some(url.trim().to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
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
        // With empty tag set, all tags should be removed
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
}
