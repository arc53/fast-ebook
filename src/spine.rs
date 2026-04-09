/// A single item reference in the reading order.
#[derive(Debug, Clone)]
pub struct SpineItem {
    pub idref: String,
    pub linear: bool,
}

/// Parse the <spine> element. Returns (items, optional NCX id).
pub fn parse_spine(spine_node: roxmltree::Node) -> (Vec<SpineItem>, Option<String>) {
    let toc_id = spine_node.attribute("toc").map(|s| s.to_string());

    let items: Vec<SpineItem> = spine_node
        .children()
        .filter(|c| c.is_element() && c.tag_name().name() == "itemref")
        .filter_map(|child| {
            let idref = child.attribute("idref")?.to_string();
            let linear = child.attribute("linear").unwrap_or("yes") != "no";
            Some(SpineItem { idref, linear })
        })
        .collect();

    (items, toc_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_from_xml(xml: &str) -> (Vec<SpineItem>, Option<String>) {
        let doc = roxmltree::Document::parse(xml).unwrap();
        let spine = doc
            .descendants()
            .find(|n| n.tag_name().name() == "spine")
            .unwrap();
        parse_spine(spine)
    }

    #[test]
    fn test_basic_spine() {
        let xml = r#"<package xmlns="http://www.idpf.org/2007/opf">
            <spine toc="ncx">
                <itemref idref="ch1"/>
                <itemref idref="ch2"/>
            </spine>
        </package>"#;
        let (items, toc) = parse_from_xml(xml);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].idref, "ch1");
        assert!(items[0].linear);
        assert_eq!(toc, Some("ncx".to_string()));
    }

    #[test]
    fn test_nonlinear_item() {
        let xml = r#"<package xmlns="http://www.idpf.org/2007/opf">
            <spine>
                <itemref idref="ch1"/>
                <itemref idref="appendix" linear="no"/>
            </spine>
        </package>"#;
        let (items, toc) = parse_from_xml(xml);
        assert!(items[0].linear);
        assert!(!items[1].linear);
        assert_eq!(toc, None);
    }
}
