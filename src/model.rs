use std::collections::HashMap;
use std::io::Read;
use std::sync::{Arc, OnceLock};

use crate::item_type::ItemType;
use crate::metadata::{MetadataItem, MetadataMap};
use crate::ncx::TocEntry;
use crate::spine::SpineItem;

/// Source data for lazy content loading.
pub(crate) struct LazySource {
    pub zip_data: Arc<Vec<u8>>,
    pub zip_path: String,
}

/// A single item (file) within an EPUB.
///
/// Content is loaded lazily when `lazy=True` is used with `read_epub`.
/// The `get_content()` method transparently resolves lazy content on first access.
pub struct EpubItem {
    pub id: String,
    pub href: String,
    pub media_type: String,
    pub item_type: ItemType,
    /// OPF manifest `properties` attribute (e.g. "nav scripted mathml").
    /// Preserved verbatim across read/write so EPUBCheck-required properties
    /// like `scripted`, `mathml`, `svg`, `remote-resources` survive roundtrip.
    pub properties: Option<String>,
    /// OPF manifest `media-overlay` attribute (idref to a SMIL item).
    pub media_overlay: Option<String>,
    /// OPF manifest `fallback` attribute (idref chain for foreign resources).
    pub fallback: Option<String>,
    content: OnceLock<Vec<u8>>,
    pub(crate) lazy_source: Option<LazySource>,
}

impl EpubItem {
    /// Create an item with content already loaded (eager mode / write mode).
    pub fn eager(
        id: String,
        href: String,
        media_type: String,
        item_type: ItemType,
        content: Vec<u8>,
    ) -> Self {
        let lock = OnceLock::new();
        let _ = lock.set(content);
        EpubItem {
            id,
            href,
            media_type,
            item_type,
            properties: None,
            media_overlay: None,
            fallback: None,
            content: lock,
            lazy_source: None,
        }
    }

    /// Create an item with deferred content loading (lazy mode).
    pub fn lazy(
        id: String,
        href: String,
        media_type: String,
        item_type: ItemType,
        zip_data: Arc<Vec<u8>>,
        zip_path: String,
    ) -> Self {
        EpubItem {
            id,
            href,
            media_type,
            item_type,
            properties: None,
            media_overlay: None,
            fallback: None,
            content: OnceLock::new(),
            lazy_source: Some(LazySource { zip_data, zip_path }),
        }
    }

    /// Maximum size for a single lazy-loaded entry (100 MB).
    const MAX_LAZY_SIZE: u64 = 100 * 1024 * 1024;

    /// Get item content. Loads from ZIP on first access if lazy.
    pub fn get_content(&self) -> &[u8] {
        self.content.get_or_init(|| {
            if let Some(source) = &self.lazy_source {
                let cursor = std::io::Cursor::new(source.zip_data.as_ref());
                if let Ok(mut archive) = zip::ZipArchive::new(cursor) {
                    if let Ok(mut file) = archive.by_name(&source.zip_path) {
                        let size = file.size();
                        if size > Self::MAX_LAZY_SIZE {
                            eprintln!(
                                "Warning: skipping oversized item '{}' ({} bytes)",
                                source.zip_path, size
                            );
                            return Vec::new();
                        }
                        let mut buf = Vec::with_capacity(size as usize);
                        if file.read_to_end(&mut buf).is_ok() {
                            return buf;
                        }
                    }
                }
            }
            Vec::new()
        })
    }
}

impl std::fmt::Debug for EpubItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EpubItem")
            .field("id", &self.id)
            .field("href", &self.href)
            .field("media_type", &self.media_type)
            .field("item_type", &self.item_type)
            .field("loaded", &self.content.get().is_some())
            .finish()
    }
}

impl Clone for EpubItem {
    fn clone(&self) -> Self {
        // Cloning always produces an eager item with resolved content
        let mut item = EpubItem::eager(
            self.id.clone(),
            self.href.clone(),
            self.media_type.clone(),
            self.item_type,
            self.get_content().to_vec(),
        );
        item.properties = self.properties.clone();
        item.media_overlay = self.media_overlay.clone();
        item.fallback = self.fallback.clone();
        item
    }
}

/// A parsed EPUB book.
pub struct EpubBook {
    pub metadata: MetadataMap,
    pub items: Vec<Arc<EpubItem>>,
    pub spine: Vec<SpineItem>,
    pub toc: Vec<TocEntry>,
    /// Source EPUB version string from `<package version="...">`. Defaults
    /// to "3.0" for books constructed via the builder API. The writer uses
    /// this to decide whether to emit EPUB2- or EPUB3-flavored OPF.
    pub version: String,
    // Fast lookup indexes
    pub(crate) id_index: HashMap<String, usize>,
    pub(crate) href_index: HashMap<String, usize>,
}

impl Default for EpubBook {
    fn default() -> Self {
        EpubBook {
            metadata: MetadataMap::new(),
            items: Vec::new(),
            spine: Vec::new(),
            toc: Vec::new(),
            version: "3.0".to_string(),
            id_index: HashMap::new(),
            href_index: HashMap::new(),
        }
    }
}

impl EpubBook {
    /// Build a new EpubBook, constructing the lookup indexes.
    pub fn new(
        metadata: MetadataMap,
        items: Vec<Arc<EpubItem>>,
        spine: Vec<SpineItem>,
        toc: Vec<TocEntry>,
    ) -> Self {
        Self::new_with_version(metadata, items, spine, toc, "3.0".to_string())
    }

    /// Build a new EpubBook with an explicit source version.
    pub fn new_with_version(
        metadata: MetadataMap,
        items: Vec<Arc<EpubItem>>,
        spine: Vec<SpineItem>,
        toc: Vec<TocEntry>,
        version: String,
    ) -> Self {
        let mut id_index = HashMap::with_capacity(items.len());
        let mut href_index = HashMap::with_capacity(items.len());

        for (i, item) in items.iter().enumerate() {
            id_index.insert(item.id.clone(), i);
            href_index.insert(item.href.clone(), i);
        }

        EpubBook {
            metadata,
            items,
            spine,
            toc,
            version,
            id_index,
            href_index,
        }
    }

    pub fn get_item_by_id(&self, id: &str) -> Option<Arc<EpubItem>> {
        self.id_index.get(id).map(|&i| Arc::clone(&self.items[i]))
    }

    pub fn get_item_by_href(&self, href: &str) -> Option<Arc<EpubItem>> {
        self.href_index
            .get(href)
            .map(|&i| Arc::clone(&self.items[i]))
    }

    /// Add an item to the book, updating lookup indexes.
    pub fn add_item(&mut self, item: Arc<EpubItem>) {
        let idx = self.items.len();
        self.id_index.insert(item.id.clone(), idx);
        self.href_index.insert(item.href.clone(), idx);
        self.items.push(item);
    }

    /// Set a metadata field (replaces existing values for that ns+field).
    pub fn set_metadata(
        &mut self,
        ns: &str,
        field: &str,
        value: &str,
        attrs: HashMap<String, String>,
    ) {
        let entries = self
            .metadata
            .entry(ns.to_string())
            .or_default()
            .entry(field.to_string())
            .or_default();
        entries.clear();
        entries.push(MetadataItem {
            value: value.to_string(),
            attributes: attrs,
        });
    }

    /// Add a metadata value (appends, does not replace).
    pub fn add_metadata(
        &mut self,
        ns: &str,
        field: &str,
        value: &str,
        attrs: HashMap<String, String>,
    ) {
        self.metadata
            .entry(ns.to_string())
            .or_default()
            .entry(field.to_string())
            .or_default()
            .push(MetadataItem {
                value: value.to_string(),
                attributes: attrs,
            });
    }

    /// Get first value for a metadata field, or None.
    pub fn get_metadata_value(&self, ns: &str, field: &str) -> Option<&str> {
        self.metadata
            .get(ns)
            .and_then(|m| m.get(field))
            .and_then(|v| v.first())
            .map(|item| item.value.as_str())
    }
}
