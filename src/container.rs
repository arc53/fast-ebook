use crate::errors::EpubError;

const CONTAINER_NS: &str = "urn:oasis:names:tc:opendocument:xmlns:container";

/// Parse META-INF/container.xml and return the full-path to the OPF file.
pub fn parse_container(xml: &str) -> Result<String, EpubError> {
    let doc = roxmltree::Document::parse(xml)?;

    for node in doc.descendants() {
        if node.tag_name().namespace() == Some(CONTAINER_NS) && node.tag_name().name() == "rootfile"
        {
            if let Some(path) = node.attribute("full-path") {
                return Ok(path.to_string());
            }
        }
    }

    Err(EpubError::MissingRootfile)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_container_basic() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <container xmlns="urn:oasis:names:tc:opendocument:xmlns:container" version="1.0">
          <rootfiles>
            <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
          </rootfiles>
        </container>"#;
        assert_eq!(parse_container(xml).unwrap(), "OEBPS/content.opf");
    }

    #[test]
    fn test_parse_container_root_level_opf() {
        let xml = r#"<?xml version="1.0"?>
        <container xmlns="urn:oasis:names:tc:opendocument:xmlns:container" version="1.0">
          <rootfiles>
            <rootfile full-path="content.opf" media-type="application/oebps-package+xml"/>
          </rootfiles>
        </container>"#;
        assert_eq!(parse_container(xml).unwrap(), "content.opf");
    }

    #[test]
    fn test_parse_container_missing_rootfile() {
        let xml = r#"<?xml version="1.0"?>
        <container xmlns="urn:oasis:names:tc:opendocument:xmlns:container" version="1.0">
          <rootfiles/>
        </container>"#;
        assert!(parse_container(xml).is_err());
    }
}
