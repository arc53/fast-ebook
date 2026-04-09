use std::collections::HashMap;
use std::sync::Arc;

use pyo3::prelude::*;

use crate::item_type::{guess_media_type, ItemType};
use crate::model::{EpubBook, EpubItem};
use crate::ncx::TocEntry;
use crate::spine::SpineItem;

#[pyclass(name = "EpubBook", subclass)]
pub struct PyEpubBook {
    pub(crate) inner: EpubBook,
}

#[pymethods]
impl PyEpubBook {
    #[new]
    fn new() -> Self {
        PyEpubBook {
            inner: EpubBook::default(),
        }
    }

    // --- Read methods (unchanged from Phase 1) ---

    /// Return all items in the book.
    fn get_items(&self) -> Vec<PyEpubItem> {
        self.inner
            .items
            .iter()
            .map(|item| PyEpubItem {
                inner: Arc::clone(item),
            })
            .collect()
    }

    /// Return items matching the given type constant (e.g., ITEM_IMAGE = 1).
    fn get_items_of_type(&self, item_type: u8) -> Vec<PyEpubItem> {
        self.inner
            .items
            .iter()
            .filter(|item| item.item_type as u8 == item_type)
            .map(|item| PyEpubItem {
                inner: Arc::clone(item),
            })
            .collect()
    }

    /// Get an item by its manifest ID.
    fn get_item_with_id(&self, item_id: &str) -> Option<PyEpubItem> {
        self.inner
            .get_item_by_id(item_id)
            .map(|item| PyEpubItem { inner: item })
    }

    /// Get an item by its href.
    fn get_item_with_href(&self, href: &str) -> Option<PyEpubItem> {
        self.inner
            .get_item_by_href(href)
            .map(|item| PyEpubItem { inner: item })
    }

    /// Get metadata values for a given namespace and field.
    /// Returns list of (value, attributes_dict) tuples.
    fn get_metadata(&self, namespace: &str, name: &str) -> Vec<(String, HashMap<String, String>)> {
        self.inner
            .metadata
            .get(namespace)
            .and_then(|ns_map| ns_map.get(name))
            .map(|items| {
                items
                    .iter()
                    .map(|item| (item.value.clone(), item.attributes.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get the spine as a list of (idref, linear) tuples.
    fn get_spine(&self) -> Vec<(String, bool)> {
        self.inner
            .spine
            .iter()
            .map(|s| (s.idref.clone(), s.linear))
            .collect()
    }

    /// Table of contents entries (getter).
    #[getter]
    fn toc(&self) -> Vec<PyTocEntry> {
        self.inner.toc.iter().map(PyTocEntry::from_entry).collect()
    }

    fn __repr__(&self) -> String {
        let title = self
            .inner
            .metadata
            .get("DC")
            .and_then(|dc| dc.get("title"))
            .and_then(|titles| titles.first())
            .map(|t| t.value.as_str())
            .unwrap_or("<untitled>");
        format!("EpubBook(title='{}')", title)
    }

    /// Convert the book to a single Markdown string (spine order).
    fn to_markdown(&self) -> String {
        crate::markdown::book_to_markdown(&self.inner)
    }

    fn __enter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __exit__(
        &self,
        _exc_type: Option<&Bound<'_, pyo3::types::PyAny>>,
        _exc_val: Option<&Bound<'_, pyo3::types::PyAny>>,
        _exc_tb: Option<&Bound<'_, pyo3::types::PyAny>>,
    ) -> bool {
        false
    }

    /// Validate the book against EPUB spec requirements.
    /// Returns a list of issue descriptions (empty = valid).
    fn validate(&self) -> Vec<String> {
        crate::validation::validate(&self.inner)
    }

    // --- Write/builder methods (Phase 2) ---

    /// Set the book's unique identifier (DC:identifier).
    fn set_identifier(&mut self, identifier: &str) {
        self.inner
            .set_metadata("DC", "identifier", identifier, HashMap::new());
    }

    /// Set the book's title (DC:title).
    fn set_title(&mut self, title: &str) {
        self.inner
            .set_metadata("DC", "title", title, HashMap::new());
    }

    /// Set the book's language (DC:language).
    fn set_language(&mut self, lang: &str) {
        self.inner
            .set_metadata("DC", "language", lang, HashMap::new());
    }

    /// Add an author (DC:creator). Appends, does not replace.
    #[pyo3(signature = (name, file_as=None, role=None, uid=None))]
    fn add_author(
        &mut self,
        name: &str,
        file_as: Option<&str>,
        role: Option<&str>,
        uid: Option<&str>,
    ) {
        let mut attrs = HashMap::new();
        if let Some(fa) = file_as {
            attrs.insert("opf:file-as".to_string(), fa.to_string());
        }
        if let Some(r) = role {
            attrs.insert("opf:role".to_string(), r.to_string());
        }
        if let Some(u) = uid {
            attrs.insert("id".to_string(), u.to_string());
        }
        self.inner.add_metadata("DC", "creator", name, attrs);
    }

    /// Add arbitrary metadata.
    #[pyo3(signature = (namespace, name, value, others=None))]
    fn add_metadata(
        &mut self,
        namespace: &str,
        name: &str,
        value: &str,
        others: Option<HashMap<String, String>>,
    ) {
        self.inner
            .add_metadata(namespace, name, value, others.unwrap_or_default());
    }

    /// Add a raw item to the book (called from Python convenience classes).
    #[pyo3(signature = (id, href, media_type, content, item_type, properties=None))]
    fn add_item_raw(
        &mut self,
        id: &str,
        href: &str,
        media_type: &str,
        content: Vec<u8>,
        item_type: u8,
        properties: Option<&str>,
    ) {
        let mut it = ItemType::from_u8(item_type);
        // Override type based on properties
        if let Some(props) = properties {
            if props.contains("nav") {
                it = ItemType::Navigation;
            } else if props.contains("cover-image") {
                it = ItemType::Cover;
            }
        }
        let item = Arc::new(EpubItem::eager(
            id.to_string(),
            href.to_string(),
            media_type.to_string(),
            it,
            content,
        ));
        self.inner.add_item(item);
    }

    /// Set the cover image.
    fn set_cover(&mut self, file_name: &str, content: Vec<u8>) {
        let media_type = guess_media_type(file_name);
        let item = Arc::new(EpubItem::eager(
            "cover-image".to_string(),
            file_name.to_string(),
            media_type,
            ItemType::Cover,
            content,
        ));
        self.inner.add_item(item);
        self.inner
            .add_metadata("OPF", "cover", "cover-image", HashMap::new());
    }

    /// Set table of contents from normalized TocEntry list (called from Python).
    fn _set_toc_from_entries(&mut self, entries: Vec<PyTocEntry>) {
        self.inner.toc = entries.into_iter().map(|e| e.into_toc_entry()).collect();
    }

    /// Set spine from normalized (idref, linear) list (called from Python).
    fn _set_spine_from_entries(&mut self, entries: Vec<(String, bool)>) {
        self.inner.spine = entries
            .into_iter()
            .map(|(idref, linear)| SpineItem { idref, linear })
            .collect();
    }
}

#[pyclass(name = "EpubItem")]
pub struct PyEpubItem {
    pub(crate) inner: Arc<EpubItem>,
}

#[pymethods]
impl PyEpubItem {
    /// Return the raw content as bytes.
    fn get_content<'py>(&self, py: Python<'py>) -> Bound<'py, pyo3::types::PyBytes> {
        pyo3::types::PyBytes::new(py, self.inner.get_content())
    }

    /// Return the item type as a u8 constant.
    fn get_type(&self) -> u8 {
        self.inner.item_type as u8
    }

    /// Return the item's href (file name within the EPUB).
    fn get_name(&self) -> &str {
        &self.inner.href
    }

    /// Return the item's manifest ID.
    fn get_id(&self) -> &str {
        &self.inner.id
    }

    /// Return the item's media type.
    fn get_media_type(&self) -> &str {
        &self.inner.media_type
    }

    /// For XHTML/HTML items, extract text content (strips tags).
    fn get_text(&self) -> Option<String> {
        if self.inner.item_type != ItemType::Document
            && self.inner.item_type != ItemType::Navigation
        {
            return None;
        }
        let content = String::from_utf8_lossy(self.inner.get_content());
        Some(strip_html_tags(&content))
    }

    fn __repr__(&self) -> String {
        format!(
            "EpubItem(id='{}', href='{}', type={})",
            self.inner.id, self.inner.href, self.inner.item_type as u8
        )
    }
}

#[pyclass(name = "TocEntry")]
#[derive(Clone)]
pub struct PyTocEntry {
    #[pyo3(get)]
    pub title: String,
    #[pyo3(get)]
    pub href: String,
    #[pyo3(get)]
    pub children: Vec<PyTocEntry>,
}

#[pymethods]
impl PyTocEntry {
    #[new]
    #[pyo3(signature = (title, href, children=Vec::new()))]
    fn new(title: String, href: String, children: Vec<PyTocEntry>) -> Self {
        PyTocEntry {
            title,
            href,
            children,
        }
    }

    fn __repr__(&self) -> String {
        format!("TocEntry(title='{}', href='{}')", self.title, self.href)
    }
}

impl PyTocEntry {
    pub(crate) fn from_entry(entry: &TocEntry) -> Self {
        PyTocEntry {
            title: entry.title.clone(),
            href: entry.href.clone(),
            children: entry.children.iter().map(PyTocEntry::from_entry).collect(),
        }
    }

    fn into_toc_entry(self) -> TocEntry {
        TocEntry {
            title: self.title,
            href: self.href,
            children: self
                .children
                .into_iter()
                .map(|c| c.into_toc_entry())
                .collect(),
        }
    }
}

/// Simple HTML tag stripper for get_text(). Extracts text content only.
fn strip_html_tags(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;

    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }

    // Normalize whitespace
    let mut prev_whitespace = false;
    let normalized: String = result
        .chars()
        .filter_map(|c| {
            if c.is_whitespace() {
                if prev_whitespace {
                    None
                } else {
                    prev_whitespace = true;
                    Some(' ')
                }
            } else {
                prev_whitespace = false;
                Some(c)
            }
        })
        .collect();

    normalized.trim().to_string()
}
