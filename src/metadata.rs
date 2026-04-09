use std::collections::HashMap;

const DC_NS: &str = "http://purl.org/dc/elements/1.1/";

/// A single metadata value with its attributes.
#[derive(Debug, Clone)]
pub struct MetadataItem {
    pub value: String,
    pub attributes: HashMap<String, String>,
}

/// Metadata organized as namespace → field → list of values.
pub type MetadataMap = HashMap<String, HashMap<String, Vec<MetadataItem>>>;

/// Parse the <metadata> element of the OPF document.
pub fn parse_metadata(metadata_node: roxmltree::Node) -> MetadataMap {
    let mut map: MetadataMap = HashMap::new();

    for child in metadata_node.children() {
        if !child.is_element() {
            continue;
        }

        let tag_name = child.tag_name();
        let ns = tag_name.namespace();
        let local_name = tag_name.name();

        // Extract text content
        let value = child
            .text()
            .or_else(|| {
                // Some elements have text in a child text node
                child
                    .children()
                    .find(|c| c.is_text())
                    .and_then(|c| c.text())
            })
            .unwrap_or("")
            .to_string();

        // Collect all attributes
        let mut attrs = HashMap::new();
        for attr in child.attributes() {
            let attr_name = if let Some(attr_ns) = attr.namespace() {
                // Prefix with a short namespace identifier
                let prefix = namespace_prefix(attr_ns);
                format!("{prefix}:{}", attr.name())
            } else {
                attr.name().to_string()
            };
            attrs.insert(attr_name, attr.value().to_string());
        }

        if ns == Some(DC_NS) {
            // Dublin Core element
            map.entry("DC".to_string())
                .or_default()
                .entry(local_name.to_string())
                .or_default()
                .push(MetadataItem {
                    value,
                    attributes: attrs,
                });
        } else if local_name == "meta" {
            // OPF <meta> element — can be EPUB2 (<meta name="..." content="..."/>)
            // or EPUB3 (<meta property="...">value</meta>)
            if let Some(property) = child.attribute("property") {
                // EPUB3 style
                let meta_value = if value.is_empty() {
                    child.attribute("content").unwrap_or("").to_string()
                } else {
                    value
                };
                map.entry("OPF".to_string())
                    .or_default()
                    .entry(property.to_string())
                    .or_default()
                    .push(MetadataItem {
                        value: meta_value,
                        attributes: attrs,
                    });
            } else if let (Some(name), Some(content)) =
                (child.attribute("name"), child.attribute("content"))
            {
                // EPUB2 style: <meta name="cover" content="cover-image"/>
                map.entry("OPF".to_string())
                    .or_default()
                    .entry(name.to_string())
                    .or_default()
                    .push(MetadataItem {
                        value: content.to_string(),
                        attributes: attrs,
                    });
            }
        }
    }

    map
}

/// Map well-known attribute namespaces to short prefixes.
fn namespace_prefix(ns: &str) -> &str {
    match ns {
        "http://www.idpf.org/2007/opf" => "opf",
        "http://www.w3.org/XML/1998/namespace" => "xml",
        _ => "ns",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_metadata_from_xml(xml: &str) -> MetadataMap {
        let doc = roxmltree::Document::parse(xml).unwrap();
        let metadata_node = doc
            .descendants()
            .find(|n| n.tag_name().name() == "metadata")
            .unwrap();
        parse_metadata(metadata_node)
    }

    #[test]
    fn test_dc_title() {
        let xml = r#"<package xmlns="http://www.idpf.org/2007/opf">
            <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
                <dc:title>Test Book</dc:title>
            </metadata>
        </package>"#;
        let meta = parse_metadata_from_xml(xml);
        let titles = &meta["DC"]["title"];
        assert_eq!(titles.len(), 1);
        assert_eq!(titles[0].value, "Test Book");
    }

    #[test]
    fn test_dc_creator_with_attributes() {
        let xml = r#"<package xmlns="http://www.idpf.org/2007/opf" xmlns:opf="http://www.idpf.org/2007/opf">
            <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
                <dc:creator opf:role="aut" opf:file-as="Doe, John">John Doe</dc:creator>
            </metadata>
        </package>"#;
        let meta = parse_metadata_from_xml(xml);
        let creators = &meta["DC"]["creator"];
        assert_eq!(creators[0].value, "John Doe");
        assert_eq!(creators[0].attributes["opf:role"], "aut");
        assert_eq!(creators[0].attributes["opf:file-as"], "Doe, John");
    }

    #[test]
    fn test_multiple_dc_fields() {
        let xml = r#"<package xmlns="http://www.idpf.org/2007/opf">
            <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
                <dc:title>Test</dc:title>
                <dc:language>en</dc:language>
                <dc:identifier id="uid">test-123</dc:identifier>
            </metadata>
        </package>"#;
        let meta = parse_metadata_from_xml(xml);
        assert_eq!(meta["DC"]["title"][0].value, "Test");
        assert_eq!(meta["DC"]["language"][0].value, "en");
        assert_eq!(meta["DC"]["identifier"][0].value, "test-123");
    }

    #[test]
    fn test_opf_meta_epub2() {
        let xml = r#"<package xmlns="http://www.idpf.org/2007/opf">
            <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
                <dc:title>Test</dc:title>
                <meta name="cover" content="cover-image"/>
            </metadata>
        </package>"#;
        let meta = parse_metadata_from_xml(xml);
        assert_eq!(meta["OPF"]["cover"][0].value, "cover-image");
    }

    #[test]
    fn test_opf_meta_epub3() {
        let xml = r#"<package xmlns="http://www.idpf.org/2007/opf">
            <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
                <dc:title>Test</dc:title>
                <meta property="dcterms:modified">2024-01-01T00:00:00Z</meta>
            </metadata>
        </package>"#;
        let meta = parse_metadata_from_xml(xml);
        assert_eq!(
            meta["OPF"]["dcterms:modified"][0].value,
            "2024-01-01T00:00:00Z"
        );
    }
}
