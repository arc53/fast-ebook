use crate::errors::EpubError;
use crate::ncx::TocEntry;

const EPUB_OPS_NS: &str = "http://www.idpf.org/2007/ops";

/// Parse an EPUB3 Navigation Document (XHTML with <nav epub:type="toc">).
pub fn parse_nav(xhtml: &str) -> Result<Vec<TocEntry>, EpubError> {
    let doc = roxmltree::Document::parse(xhtml)?;

    // Find <nav> with epub:type="toc"
    let nav_node = doc.descendants().find(|n| {
        n.tag_name().name() == "nav" && n.attribute((EPUB_OPS_NS, "type")) == Some("toc")
    });

    // Fallback: look for any <nav> element with a type attribute containing "toc"
    let nav_node = nav_node.or_else(|| {
        doc.descendants().find(|n| {
            n.tag_name().name() == "nav"
                && n.attributes()
                    .any(|a| a.name() == "type" && a.value() == "toc")
        })
    });

    match nav_node {
        Some(nav) => {
            // Find the <ol> inside the nav
            let ol = nav
                .children()
                .find(|c| c.is_element() && c.tag_name().name() == "ol");
            match ol {
                Some(ol_node) => Ok(parse_ol(ol_node)),
                None => Ok(Vec::new()),
            }
        }
        None => Ok(Vec::new()),
    }
}

/// Maximum recursion depth for Nav parsing.
const MAX_NAV_DEPTH: usize = 100;

/// Recursively parse an <ol> element into TocEntry list.
fn parse_ol(ol: roxmltree::Node) -> Vec<TocEntry> {
    parse_ol_depth(ol, 0)
}

fn parse_ol_depth(ol: roxmltree::Node, depth: usize) -> Vec<TocEntry> {
    if depth >= MAX_NAV_DEPTH {
        return Vec::new();
    }
    let mut entries = Vec::new();

    for li in ol.children() {
        if !li.is_element() || li.tag_name().name() != "li" {
            continue;
        }

        // Find <a> element for title and href
        let anchor = li
            .children()
            .find(|c| c.is_element() && c.tag_name().name() == "a");

        let (title, href) = match anchor {
            Some(a) => {
                let href = a.attribute("href").unwrap_or("").to_string();
                // Collect all text content from the anchor (handles nested <span> etc.)
                let title = collect_text(a);
                (title, href)
            }
            None => {
                // Some navs use <span> instead of <a> for section headings
                let span = li
                    .children()
                    .find(|c| c.is_element() && c.tag_name().name() == "span");
                let title = span.map(collect_text).unwrap_or_default();
                (title, String::new())
            }
        };

        // Look for nested <ol> for children
        let children = li
            .children()
            .find(|c| c.is_element() && c.tag_name().name() == "ol")
            .map(|ol_node| parse_ol_depth(ol_node, depth + 1))
            .unwrap_or_default();

        entries.push(TocEntry {
            title,
            href,
            children,
        });
    }

    entries
}

/// Collect all text content from a node's children (not the node itself).
fn collect_text(node: roxmltree::Node) -> String {
    let mut text = String::new();
    collect_text_recursive(node, &mut text);
    text.trim().to_string()
}

fn collect_text_recursive(node: roxmltree::Node, buf: &mut String) {
    for child in node.children() {
        if child.is_text() {
            if let Some(t) = child.text() {
                buf.push_str(t);
            }
        } else if child.is_element() {
            collect_text_recursive(child, buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_nav() {
        let xhtml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops">
        <body>
          <nav epub:type="toc">
            <ol>
              <li><a href="chapter1.xhtml">Chapter 1</a></li>
              <li><a href="chapter2.xhtml">Chapter 2</a></li>
            </ol>
          </nav>
        </body>
        </html>"#;
        let entries = parse_nav(xhtml).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].title, "Chapter 1");
        assert_eq!(entries[0].href, "chapter1.xhtml");
    }

    #[test]
    fn test_nested_nav() {
        let xhtml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops">
        <body>
          <nav epub:type="toc">
            <ol>
              <li>
                <a href="part1.xhtml">Part 1</a>
                <ol>
                  <li><a href="ch1.xhtml">Chapter 1</a></li>
                  <li><a href="ch2.xhtml">Chapter 2</a></li>
                </ol>
              </li>
            </ol>
          </nav>
        </body>
        </html>"#;
        let entries = parse_nav(xhtml).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].title, "Part 1");
        assert_eq!(entries[0].children.len(), 2);
        assert_eq!(entries[0].children[0].title, "Chapter 1");
    }

    #[test]
    fn test_nav_with_span_heading() {
        let xhtml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops">
        <body>
          <nav epub:type="toc">
            <ol>
              <li>
                <span>Section</span>
                <ol>
                  <li><a href="ch1.xhtml">Chapter 1</a></li>
                </ol>
              </li>
            </ol>
          </nav>
        </body>
        </html>"#;
        let entries = parse_nav(xhtml).unwrap();
        assert_eq!(entries[0].title, "Section");
        assert!(entries[0].href.is_empty());
        assert_eq!(entries[0].children.len(), 1);
    }
}
