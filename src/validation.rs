use std::collections::HashSet;

use crate::item_type::ItemType;
use crate::model::EpubBook;

/// Validate an EpubBook against EPUB spec requirements.
/// Returns a list of issue descriptions (empty = valid).
pub fn validate(book: &EpubBook) -> Vec<String> {
    let mut issues = Vec::new();

    // Required DC metadata
    if book.get_metadata_value("DC", "identifier").is_none() {
        issues.push("Missing required metadata: DC:identifier".to_string());
    }
    if book.get_metadata_value("DC", "title").is_none() {
        issues.push("Missing required metadata: DC:title".to_string());
    }
    if book.get_metadata_value("DC", "language").is_none() {
        issues.push("Missing required metadata: DC:language".to_string());
    }

    // Spine
    if book.spine.is_empty() {
        issues.push("Spine is empty — at least one content document is required".to_string());
    }

    // Spine references must point to existing items
    for entry in &book.spine {
        if !book.id_index.contains_key(&entry.idref) {
            issues.push(format!(
                "Spine references item '{}' which is not in the manifest",
                entry.idref
            ));
        }
    }

    // Duplicate item IDs
    let mut seen_ids = HashSet::new();
    for item in &book.items {
        if !seen_ids.insert(&item.id) {
            issues.push(format!("Duplicate item ID: '{}'", item.id));
        }
    }

    // Duplicate item hrefs
    let mut seen_hrefs = HashSet::new();
    for item in &book.items {
        if !seen_hrefs.insert(&item.href) {
            issues.push(format!("Duplicate item href: '{}'", item.href));
        }
    }

    // Navigation item
    let has_nav = book
        .items
        .iter()
        .any(|i| i.item_type == ItemType::Navigation);
    if !has_nav {
        issues.push("No navigation item (NCX or Nav document) found".to_string());
    }

    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::EpubItem;
    use crate::spine::SpineItem;
    use std::collections::HashMap;
    use std::sync::Arc;

    fn make_valid_book() -> EpubBook {
        let mut book = EpubBook::default();
        book.set_metadata("DC", "identifier", "id", HashMap::new());
        book.set_metadata("DC", "title", "Title", HashMap::new());
        book.set_metadata("DC", "language", "en", HashMap::new());
        book.add_item(Arc::new(EpubItem::eager(
            "ch1".to_string(),
            "ch1.xhtml".to_string(),
            "application/xhtml+xml".to_string(),
            ItemType::Document,
            Vec::new(),
        )));
        book.add_item(Arc::new(EpubItem::eager(
            "nav".to_string(),
            "nav.xhtml".to_string(),
            "application/xhtml+xml".to_string(),
            ItemType::Navigation,
            Vec::new(),
        )));
        book.spine = vec![SpineItem {
            idref: "ch1".to_string(),
            linear: true,
        }];
        book
    }

    #[test]
    fn test_valid_book_has_no_issues() {
        let book = make_valid_book();
        assert!(validate(&book).is_empty());
    }

    #[test]
    fn test_missing_identifier() {
        let mut book = make_valid_book();
        book.metadata.get_mut("DC").unwrap().remove("identifier");
        let issues = validate(&book);
        assert!(issues.iter().any(|i| i.contains("identifier")));
    }

    #[test]
    fn test_dangling_spine_ref() {
        let mut book = make_valid_book();
        book.spine.push(SpineItem {
            idref: "nonexistent".to_string(),
            linear: true,
        });
        let issues = validate(&book);
        assert!(issues.iter().any(|i| i.contains("nonexistent")));
    }

    #[test]
    fn test_no_navigation() {
        let mut book = make_valid_book();
        // Remove nav item
        book.items.retain(|i| i.item_type != ItemType::Navigation);
        let issues = validate(&book);
        assert!(issues.iter().any(|i| i.contains("navigation")));
    }
}
