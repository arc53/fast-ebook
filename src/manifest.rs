use std::collections::HashMap;

/// A single item from the OPF <manifest>.
#[derive(Debug, Clone)]
pub struct ManifestItem {
    pub id: String,
    pub href: String,
    pub media_type: String,
    pub properties: Option<String>,
}

/// Parse the <manifest> element, returning a map of id → ManifestItem.
pub fn parse_manifest(manifest_node: roxmltree::Node) -> HashMap<String, ManifestItem> {
    let mut items = HashMap::new();

    for child in manifest_node.children() {
        if !child.is_element() || child.tag_name().name() != "item" {
            continue;
        }

        let id = match child.attribute("id") {
            Some(id) => id.to_string(),
            None => continue,
        };
        let href = match child.attribute("href") {
            Some(href) => href.to_string(),
            None => continue,
        };
        let media_type = child.attribute("media-type").unwrap_or("").to_string();
        let properties = child.attribute("properties").map(|s| s.to_string());

        items.insert(
            id.clone(),
            ManifestItem {
                id,
                href,
                media_type,
                properties,
            },
        );
    }

    items
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_from_xml(xml: &str) -> HashMap<String, ManifestItem> {
        let doc = roxmltree::Document::parse(xml).unwrap();
        let manifest = doc
            .descendants()
            .find(|n| n.tag_name().name() == "manifest")
            .unwrap();
        parse_manifest(manifest)
    }

    #[test]
    fn test_basic_manifest() {
        let xml = r#"<package xmlns="http://www.idpf.org/2007/opf">
            <manifest>
                <item id="ch1" href="chapter1.xhtml" media-type="application/xhtml+xml"/>
                <item id="css" href="style.css" media-type="text/css"/>
                <item id="img1" href="cover.jpg" media-type="image/jpeg" properties="cover-image"/>
            </manifest>
        </package>"#;
        let items = parse_from_xml(xml);
        assert_eq!(items.len(), 3);
        assert_eq!(items["ch1"].href, "chapter1.xhtml");
        assert_eq!(items["css"].media_type, "text/css");
        assert_eq!(items["img1"].properties.as_deref(), Some("cover-image"));
    }

    #[test]
    fn test_nav_item() {
        let xml = r#"<package xmlns="http://www.idpf.org/2007/opf">
            <manifest>
                <item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>
            </manifest>
        </package>"#;
        let items = parse_from_xml(xml);
        assert_eq!(items["nav"].properties.as_deref(), Some("nav"));
    }
}
