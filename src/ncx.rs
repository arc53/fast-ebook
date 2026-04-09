use crate::errors::EpubError;

/// A table of contents entry (used by both NCX and Nav parsers).
#[derive(Debug, Clone)]
pub struct TocEntry {
    pub title: String,
    pub href: String,
    pub children: Vec<TocEntry>,
}

/// Parse an NCX document (EPUB2 table of contents).
pub fn parse_ncx(xml: &str) -> Result<Vec<TocEntry>, EpubError> {
    let doc = roxmltree::Document::parse(xml)?;

    // Find <navMap>
    let nav_map = doc.descendants().find(|n| n.tag_name().name() == "navMap");

    match nav_map {
        Some(node) => Ok(parse_nav_points(node)),
        None => Ok(Vec::new()),
    }
}

/// Maximum recursion depth for ToC parsing (prevents stack overflow on malicious input).
const MAX_TOC_DEPTH: usize = 100;

/// Recursively parse <navPoint> elements.
fn parse_nav_points(parent: roxmltree::Node) -> Vec<TocEntry> {
    parse_nav_points_depth(parent, 0)
}

fn parse_nav_points_depth(parent: roxmltree::Node, depth: usize) -> Vec<TocEntry> {
    if depth >= MAX_TOC_DEPTH {
        return Vec::new();
    }
    let mut entries = Vec::new();

    for child in parent.children() {
        if !child.is_element() || child.tag_name().name() != "navPoint" {
            continue;
        }

        let title = child
            .descendants()
            .find(|n| n.tag_name().name() == "navLabel")
            .and_then(|label| label.descendants().find(|n| n.tag_name().name() == "text"))
            .and_then(|text| text.text())
            .unwrap_or("")
            .to_string();

        let href = child
            .descendants()
            .find(|n| n.tag_name().name() == "content")
            .and_then(|c| c.attribute("src"))
            .unwrap_or("")
            .to_string();

        let children = parse_nav_points_depth(child, depth + 1);

        entries.push(TocEntry {
            title,
            href,
            children,
        });
    }

    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_ncx() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <ncx xmlns="http://www.daisy.org/z3986/2005/ncx/">
          <navMap>
            <navPoint id="np1" playOrder="1">
              <navLabel><text>Chapter 1</text></navLabel>
              <content src="chapter1.xhtml"/>
            </navPoint>
            <navPoint id="np2" playOrder="2">
              <navLabel><text>Chapter 2</text></navLabel>
              <content src="chapter2.xhtml"/>
            </navPoint>
          </navMap>
        </ncx>"#;
        let entries = parse_ncx(xml).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].title, "Chapter 1");
        assert_eq!(entries[0].href, "chapter1.xhtml");
        assert_eq!(entries[1].title, "Chapter 2");
    }

    #[test]
    fn test_nested_ncx() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <ncx xmlns="http://www.daisy.org/z3986/2005/ncx/">
          <navMap>
            <navPoint id="part1" playOrder="1">
              <navLabel><text>Part 1</text></navLabel>
              <content src="part1.xhtml"/>
              <navPoint id="ch1" playOrder="2">
                <navLabel><text>Chapter 1</text></navLabel>
                <content src="chapter1.xhtml"/>
              </navPoint>
              <navPoint id="ch2" playOrder="3">
                <navLabel><text>Chapter 2</text></navLabel>
                <content src="chapter2.xhtml"/>
              </navPoint>
            </navPoint>
          </navMap>
        </ncx>"#;
        let entries = parse_ncx(xml).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].title, "Part 1");
        assert_eq!(entries[0].children.len(), 2);
        assert_eq!(entries[0].children[0].title, "Chapter 1");
    }

    #[test]
    fn test_empty_navmap() {
        let xml = r#"<?xml version="1.0"?>
        <ncx xmlns="http://www.daisy.org/z3986/2005/ncx/">
          <navMap/>
        </ncx>"#;
        let entries = parse_ncx(xml).unwrap();
        assert!(entries.is_empty());
    }
}
