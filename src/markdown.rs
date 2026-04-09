//! Convert EPUB HTML content to Markdown.

use crate::item_type::ItemType;
use crate::model::EpubBook;

/// Convert an entire EpubBook to a single Markdown string.
/// Follows spine order, skipping non-document items.
pub fn book_to_markdown(book: &EpubBook) -> String {
    let title = book.get_metadata_value("DC", "title").unwrap_or("Untitled");
    let author = book
        .get_metadata_value("DC", "creator")
        .unwrap_or("Unknown");

    // Estimate capacity: ~1 byte per byte of XHTML content
    let total_estimate: usize = book
        .items
        .iter()
        .filter(|i| i.item_type == ItemType::Document)
        .map(|i| i.get_content().len())
        .sum();

    let mut md = String::with_capacity(total_estimate + 256);

    // Header
    md.push_str("# ");
    md.push_str(title);
    md.push_str("\n\n**");
    md.push_str(author);
    md.push_str("**\n\n---\n\n");

    // Build id -> item index for spine lookup
    let docs_by_id: std::collections::HashMap<&str, &crate::model::EpubItem> = book
        .items
        .iter()
        .filter(|i| i.item_type == ItemType::Document)
        .map(|i| (i.id.as_str(), i.as_ref()))
        .collect();

    let mut first = true;
    for spine_entry in &book.spine {
        if let Some(item) = docs_by_id.get(spine_entry.idref.as_str()) {
            let content = item.get_content();
            let html = String::from_utf8_lossy(content);
            let chapter_md = html_to_markdown(&html);
            if !chapter_md.is_empty() {
                if !first {
                    md.push_str("\n\n---\n\n");
                }
                md.push_str(&chapter_md);
                first = false;
            }
        }
    }

    md
}

/// Convert an HTML/XHTML string to Markdown.
pub fn html_to_markdown(html: &str) -> String {
    // Extract body content
    let body = extract_body(html);

    let mut md = String::with_capacity(body.len());
    let mut chars = body.chars().peekable();

    while let Some(&ch) = chars.peek() {
        if ch == '<' {
            // Parse tag
            let tag = consume_tag(&mut chars);
            handle_tag(&tag, &mut md);
        } else if ch == '&' {
            // HTML entity
            let entity = consume_entity(&mut chars);
            md.push_str(&decode_entity(&entity));
        } else {
            chars.next();
            md.push(ch);
        }
    }

    // Normalize whitespace
    normalize_whitespace(&md)
}

/// Extract content between <body> and </body> tags.
fn extract_body(html: &str) -> &str {
    let lower = html.to_lowercase();
    let start = if let Some(pos) = lower.find("<body") {
        // Find the closing > of the <body> tag
        html[pos..].find('>').map(|i| pos + i + 1).unwrap_or(0)
    } else {
        0
    };
    let end = lower.rfind("</body>").unwrap_or(html.len());
    &html[start..end]
}

/// Consume a full HTML tag from the char iterator, returning it as a string.
fn consume_tag(chars: &mut std::iter::Peekable<std::str::Chars>) -> String {
    let mut tag = String::new();
    chars.next(); // consume '<'
    for ch in chars.by_ref() {
        if ch == '>' {
            break;
        }
        tag.push(ch);
    }
    tag
}

/// Consume an HTML entity like &amp; or &#160;
fn consume_entity(chars: &mut std::iter::Peekable<std::str::Chars>) -> String {
    let mut entity = String::new();
    chars.next(); // consume '&'
    for ch in chars.by_ref() {
        if ch == ';' {
            break;
        }
        entity.push(ch);
        if entity.len() > 10 {
            break; // not a valid entity, bail
        }
    }
    entity
}

/// Handle an HTML tag and write appropriate Markdown.
fn handle_tag(tag: &str, md: &mut String) {
    let tag_lower = tag.to_lowercase();
    let tag_name = tag_lower
        .split(|c: char| c.is_whitespace())
        .next()
        .unwrap_or("");

    // Strip leading / for closing tags
    let (is_closing, name) = if let Some(n) = tag_name.strip_prefix('/') {
        (true, n)
    } else {
        (false, tag_name)
    };

    match name {
        "h1" => {
            if is_closing {
                md.push_str("\n\n");
            } else {
                ensure_newline(md);
                md.push_str("# ");
            }
        }
        "h2" => {
            if is_closing {
                md.push_str("\n\n");
            } else {
                ensure_newline(md);
                md.push_str("## ");
            }
        }
        "h3" => {
            if is_closing {
                md.push_str("\n\n");
            } else {
                ensure_newline(md);
                md.push_str("### ");
            }
        }
        "h4" => {
            if is_closing {
                md.push_str("\n\n");
            } else {
                ensure_newline(md);
                md.push_str("#### ");
            }
        }
        "h5" => {
            if is_closing {
                md.push_str("\n\n");
            } else {
                ensure_newline(md);
                md.push_str("##### ");
            }
        }
        "h6" => {
            if is_closing {
                md.push_str("\n\n");
            } else {
                ensure_newline(md);
                md.push_str("###### ");
            }
        }
        "p" | "div" => {
            if is_closing {
                md.push_str("\n\n");
            } else {
                ensure_newline(md);
            }
        }
        "br" => {
            md.push('\n');
        }
        "b" | "strong" => {
            md.push_str("**");
        }
        "i" | "em" => {
            md.push('*');
        }
        "a" => {
            if is_closing {
                // Extract href from the opening tag — we need a different approach
                // For simplicity, just close the link text
                md.push(']');
                // We can't easily get the href here in a streaming parser,
                // so we'll just output the text without link
            } else if let Some(href) = extract_attr(tag, "href") {
                md.push('[');
                // Store href — we'll need to append it after closing tag
                // For a streaming approach, push a marker
                md.push_str(&format!("\x00HREF:{}\x00", href));
            }
        }
        "li" => {
            if !is_closing {
                ensure_newline(md);
                md.push_str("- ");
            } else {
                md.push('\n');
            }
        }
        "blockquote" => {
            if !is_closing {
                ensure_newline(md);
                md.push_str("> ");
            } else {
                md.push_str("\n\n");
            }
        }
        "hr" => {
            ensure_newline(md);
            md.push_str("---\n\n");
        }
        _ => {} // ignore unknown tags
    }
}

/// Extract an attribute value from a tag string.
fn extract_attr<'a>(tag: &'a str, attr_name: &str) -> Option<&'a str> {
    let lower = tag.to_lowercase();
    let needle = format!("{}=\"", attr_name);
    if let Some(pos) = lower.find(&needle) {
        let start = pos + needle.len();
        let rest = &tag[start..];
        rest.find('"').map(|end| &rest[..end])
    } else {
        None
    }
}

/// Ensure the buffer ends with a newline (avoid double newlines before headings).
fn ensure_newline(md: &mut String) {
    if !md.is_empty() && !md.ends_with('\n') {
        md.push('\n');
    }
}

/// Decode common HTML entities.
fn decode_entity(entity: &str) -> String {
    match entity {
        "amp" => "&".to_string(),
        "lt" => "<".to_string(),
        "gt" => ">".to_string(),
        "quot" => "\"".to_string(),
        "apos" => "'".to_string(),
        "nbsp" | "#160" => " ".to_string(),
        "mdash" | "#8212" => "—".to_string(),
        "ndash" | "#8211" => "–".to_string(),
        "lsquo" | "#8216" => "\u{2018}".to_string(),
        "rsquo" | "#8217" => "\u{2019}".to_string(),
        "ldquo" | "#8220" => "\u{201C}".to_string(),
        "rdquo" | "#8221" => "\u{201D}".to_string(),
        "hellip" | "#8230" => "…".to_string(),
        e if e.starts_with('#') => {
            // Numeric entity
            let num = if let Some(hex) = e.strip_prefix("#x") {
                u32::from_str_radix(hex, 16).ok()
            } else {
                e[1..].parse::<u32>().ok()
            };
            num.and_then(char::from_u32)
                .map(|c| c.to_string())
                .unwrap_or_default()
        }
        _ => format!("&{};", entity), // preserve unknown entities
    }
}

/// Normalize whitespace: collapse runs, clean up excessive newlines, fix links.
fn normalize_whitespace(text: &str) -> String {
    // First, fix the link markers: [HREF:url text] -> [text](url)
    let text = fix_links(text);

    let mut result = String::with_capacity(text.len());
    let mut prev_newline_count = 0;
    let mut prev_space = false;

    for ch in text.chars() {
        if ch == '\n' {
            prev_newline_count += 1;
            prev_space = false;
            if prev_newline_count <= 2 {
                result.push('\n');
            }
        } else if ch.is_whitespace() {
            prev_newline_count = 0;
            if !prev_space {
                result.push(' ');
                prev_space = true;
            }
        } else {
            prev_newline_count = 0;
            prev_space = false;
            result.push(ch);
        }
    }

    // Trim each line
    result
        .lines()
        .map(|l| l.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

/// Fix link markers from streaming parse: [\0HREF:url\0text] -> [text](url)
fn fix_links(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut i = 0;
    let bytes = text.as_bytes();

    while i < bytes.len() {
        if bytes[i] == b'[' && i + 1 < bytes.len() && bytes[i + 1] == 0 {
            // Found link marker start
            if let Some(href_end) = text[i + 2..].find('\0') {
                let href_data = &text[i + 2..i + 2 + href_end];
                if let Some(href) = href_data.strip_prefix("HREF:") {
                    // Find the closing ]
                    let after_marker = i + 2 + href_end + 1;
                    if let Some(close) = text[after_marker..].find(']') {
                        let link_text = &text[after_marker..after_marker + close];
                        result.push('[');
                        result.push_str(link_text.trim());
                        result.push_str("](");
                        result.push_str(href);
                        result.push(')');
                        i = after_marker + close + 1;
                        continue;
                    }
                }
            }
        }
        result.push(text[i..].chars().next().unwrap());
        i += text[i..].chars().next().unwrap().len_utf8();
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_html() {
        let md = html_to_markdown("<html><body><h1>Title</h1><p>Hello world.</p></body></html>");
        assert!(md.contains("# Title"));
        assert!(md.contains("Hello world."));
    }

    #[test]
    fn test_bold_italic() {
        let md = html_to_markdown("<body><p>This is <b>bold</b> and <i>italic</i>.</p></body>");
        assert!(md.contains("**bold**"));
        assert!(md.contains("*italic*"));
    }

    #[test]
    fn test_headings() {
        let md = html_to_markdown("<body><h1>H1</h1><h2>H2</h2><h3>H3</h3></body>");
        assert!(md.contains("# H1"));
        assert!(md.contains("## H2"));
        assert!(md.contains("### H3"));
    }

    #[test]
    fn test_entities() {
        let md = html_to_markdown("<body><p>A &amp; B &lt; C &gt; D</p></body>");
        assert!(md.contains("A & B < C > D"));
    }

    #[test]
    fn test_list() {
        let md = html_to_markdown("<body><ul><li>One</li><li>Two</li></ul></body>");
        assert!(md.contains("- One"));
        assert!(md.contains("- Two"));
    }

    #[test]
    fn test_strips_tags() {
        let md = html_to_markdown("<body><div class=\"chapter\"><span>Text</span></div></body>");
        assert!(md.contains("Text"));
        assert!(!md.contains("<span>"));
        assert!(!md.contains("<div"));
    }

    #[test]
    fn test_empty_body() {
        let md = html_to_markdown("<html><body></body></html>");
        assert!(md.is_empty());
    }

    #[test]
    fn test_no_body_tag() {
        let md = html_to_markdown("<p>Just a paragraph</p>");
        assert!(md.contains("Just a paragraph"));
    }
}
