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



