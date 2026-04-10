use std::io::{Read, Seek};
use std::sync::Arc;

use crate::container;
use crate::errors::EpubError;
use crate::item_type::ItemType;
use crate::manifest;
use crate::metadata;
use crate::model::{EpubBook, EpubItem};
use crate::nav;
use crate::ncx;
use crate::spine;

/// Options for reading an EPUB.
#[derive(Default)]
pub struct ReadOptions {
    pub ignore_ncx: bool,
    pub ignore_nav: bool,
    pub lazy: bool,
}

/// Read and parse an EPUB file from the given path.
#[allow(dead_code)]
pub fn read_epub(path: &str) -> Result<EpubBook, EpubError> {
    read_epub_with_options(path, &ReadOptions::default())
}

/// Read and parse an EPUB file with options.
pub fn read_epub_with_options(path: &str, opts: &ReadOptions) -> Result<EpubBook, EpubError> {
    if opts.lazy {
        // Lazy mode: read entire file into memory, then parse lazily
        let data = std::fs::read(path)?;
        read_epub_from_bytes_with_options(&data, opts)
    } else {
        // Eager mode: stream from file
        let file = std::fs::File::open(path)?;
        let mut archive = zip::ZipArchive::new(file)?;
        read_epub_inner(&mut archive, opts, None)
    }
}

/// Read and parse an EPUB from raw bytes.
#[allow(dead_code)]
pub fn read_epub_from_bytes(data: &[u8]) -> Result<EpubBook, EpubError> {
    read_epub_from_bytes_with_options(data, &ReadOptions::default())
}

/// Read and parse an EPUB from raw bytes with options.
pub fn read_epub_from_bytes_with_options(
    data: &[u8],
    opts: &ReadOptions,
) -> Result<EpubBook, EpubError> {
    let zip_data = if opts.lazy {
        Some(Arc::new(data.to_vec()))
    } else {
        None
    };
    let cursor = std::io::Cursor::new(data);
    let mut archive = zip::ZipArchive::new(cursor)?;
    read_epub_inner(&mut archive, opts, zip_data)
}

/// Core EPUB reading logic, generic over any Read+Seek source.
/// When `zip_data` is Some, items are created in lazy mode.
fn read_epub_inner<R: Read + Seek>(
    archive: &mut zip::ZipArchive<R>,
    opts: &ReadOptions,
    zip_data: Option<Arc<Vec<u8>>>,
) -> Result<EpubBook, EpubError> {
    // 1. Verify mimetype (lenient — warn but don't fail)
    verify_mimetype(archive);

    // 2. Parse container.xml to find OPF path
    let container_xml = read_zip_entry(archive, "META-INF/container.xml")
        .map_err(|_| EpubError::MissingContainer)?;
    let container_str = String::from_utf8_lossy(&container_xml);
    let opf_path = container::parse_container(&container_str)?;

    // 3. Parse OPF
    let opf_xml =
        read_zip_entry(archive, &opf_path).map_err(|_| EpubError::MissingOpf(opf_path.clone()))?;
    let opf_str = String::from_utf8_lossy(&opf_xml);
    let opf_doc =
        roxmltree::Document::parse(&opf_str).map_err(|e| EpubError::InvalidOpf(e.to_string()))?;

    // Find key OPF sections
    let package = opf_doc.root_element();
    let version = package.attribute("version").unwrap_or("3.0").to_string();

    let metadata_node = package
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "metadata");

    let manifest_node = package
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "manifest");

    let spine_node = package
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "spine");

    // 4. Parse metadata, manifest, spine (tolerant of missing sections)
    let meta = metadata_node
        .map(|n| metadata::parse_metadata(n))
        .unwrap_or_default();

    let manifest_items = manifest_node
        .map(|n| manifest::parse_manifest(n))
        .unwrap_or_default();

    let (spine_items, ncx_id) = spine_node
        .map(|n| spine::parse_spine(n))
        .unwrap_or_default();

    // 5. Compute OPF base directory for resolving relative hrefs
    let opf_base = opf_path.rfind('/').map(|i| &opf_path[..=i]).unwrap_or("");

    // 6. Extract items from ZIP
    let mut items = Vec::with_capacity(manifest_items.len());
    for manifest_item in manifest_items.values() {
        let zip_path = resolve_relative(opf_base, &manifest_item.href);

        let item_type = ItemType::from_media_type(
            &manifest_item.media_type,
            manifest_item.properties.as_deref(),
        );

        let mut item = if let Some(ref zd) = zip_data {
            // Lazy mode: defer content loading
            EpubItem::lazy(
                manifest_item.id.clone(),
                manifest_item.href.clone(),
                manifest_item.media_type.clone(),
                item_type,
                Arc::clone(zd),
                zip_path,
            )
        } else {
            // Eager mode: load content now
            let content = read_zip_entry(archive, &zip_path).unwrap_or_default();
            EpubItem::eager(
                manifest_item.id.clone(),
                manifest_item.href.clone(),
                manifest_item.media_type.clone(),
                item_type,
                content,
            )
        };

        // Preserve OPF manifest attributes verbatim so the writer can emit
        // them on roundtrip (required for EPUBCheck conformance — properties
        // like `scripted`, `mathml`, `svg`, and the `media-overlay`/`fallback`
        // idref chains MUST survive round-trip).
        item.properties = manifest_item.properties.clone();
        item.media_overlay = manifest_item.media_overlay.clone();
        item.fallback = manifest_item.fallback.clone();

        items.push(Arc::new(item));
    }

    // 7. Parse table of contents
    let mut toc = Vec::new();

    // Try EPUB3 Nav document first (unless ignored)
    if !opts.ignore_nav {
        let nav_item = manifest_items
            .values()
            .find(|item| item.properties.as_ref().is_some_and(|p| p.contains("nav")));
        if let Some(nav_manifest) = nav_item {
            let nav_path = resolve_relative(opf_base, &nav_manifest.href);
            if let Ok(nav_content) = read_zip_entry(archive, &nav_path) {
                let nav_str = String::from_utf8_lossy(&nav_content);
                if let Ok(nav_toc) = nav::parse_nav(&nav_str) {
                    if !nav_toc.is_empty() {
                        toc = nav_toc;
                    }
                }
            }
        }
    }

    // Fallback to NCX (EPUB2) (unless ignored)
    if toc.is_empty() && !opts.ignore_ncx {
        if let Some(ncx_manifest_id) = &ncx_id {
            if let Some(ncx_manifest) = manifest_items.get(ncx_manifest_id) {
                let ncx_path = resolve_relative(opf_base, &ncx_manifest.href);
                if let Ok(ncx_content) = read_zip_entry(archive, &ncx_path) {
                    let ncx_str = String::from_utf8_lossy(&ncx_content);
                    if let Ok(ncx_toc) = ncx::parse_ncx(&ncx_str) {
                        toc = ncx_toc;
                    }
                }
            }
        }
    }

    // 8. Merge EPUB3 refines metadata back into the corresponding DC element
    // attributes (so opf:role / opf:file-as round-trip via the Python API).
    let mut meta = meta;
    merge_refines_into_dc(&mut meta);

    // 9. Build and return EpubBook
    Ok(EpubBook::new_with_version(
        meta,
        items,
        spine_items,
        toc,
        version,
    ))
}

/// Convert EPUB3 `<meta refines="#dc-id" property="role">val</meta>` style
/// refinements into `opf:{property}` attributes on the target DC element,
/// then strip the original meta entries. This makes EPUB3 sources surface
/// the same attribute keys (`opf:role`, `opf:file-as`, ...) as EPUB2
/// sources, so callers don't have to special-case either format.
fn merge_refines_into_dc(meta: &mut crate::metadata::MetadataMap) {
    // Build a map of dc id → list of (field, item-index) so we can find
    // refinement targets without re-walking the whole metadata each time.
    let mut id_to_dc: std::collections::HashMap<String, (String, usize)> =
        std::collections::HashMap::new();
    if let Some(dc) = meta.get("DC") {
        for (field, items) in dc {
            for (idx, item) in items.iter().enumerate() {
                if let Some(id) = item.attributes.get("id") {
                    id_to_dc.insert(id.clone(), (field.clone(), idx));
                }
            }
        }
    }
    if id_to_dc.is_empty() {
        return;
    }

    // Find refines metas pointing at known DC ids and a known opf-style
    // property. Collect (dc_id, prop, value, opf_key, opf_index) for each.
    let mut to_apply: Vec<(String, String, String, String, usize)> = Vec::new();
    if let Some(opf) = meta.get("OPF") {
        for (key, items) in opf {
            for (idx, item) in items.iter().enumerate() {
                let Some(refines) = item.attributes.get("refines") else {
                    continue;
                };
                let Some(prop) = item.attributes.get("property") else {
                    continue;
                };
                if !matches!(
                    prop.as_str(),
                    "role" | "file-as" | "display-seq" | "group-position"
                ) {
                    continue;
                }
                let id = refines.trim_start_matches('#').to_string();
                if !id_to_dc.contains_key(&id) {
                    continue;
                }
                to_apply.push((id, prop.clone(), item.value.clone(), key.clone(), idx));
            }
        }
    }
    if to_apply.is_empty() {
        return;
    }

    // Apply: inject opf:{prop} on the target DC item.
    if let Some(dc) = meta.get_mut("DC") {
        for (id, prop, val, _, _) in &to_apply {
            if let Some((field, idx)) = id_to_dc.get(id) {
                if let Some(items) = dc.get_mut(field) {
                    if let Some(item) = items.get_mut(*idx) {
                        item.attributes.insert(format!("opf:{}", prop), val.clone());
                    }
                }
            }
        }
    }

    // Remove the consumed refines metas (in reverse index order per key).
    if let Some(opf) = meta.get_mut("OPF") {
        let mut by_key: std::collections::HashMap<String, Vec<usize>> =
            std::collections::HashMap::new();
        for (_, _, _, key, idx) in to_apply {
            by_key.entry(key).or_default().push(idx);
        }
        for (key, mut indices) in by_key {
            indices.sort_unstable_by(|a, b| b.cmp(a));
            if let Some(items) = opf.get_mut(&key) {
                for idx in indices {
                    if idx < items.len() {
                        items.remove(idx);
                    }
                }
                if items.is_empty() {
                    opf.remove(&key);
                }
            }
        }
    }
}

/// Maximum size for a single ZIP entry (100 MB).
const MAX_ENTRY_SIZE: u64 = 100 * 1024 * 1024;

/// Read a file from the ZIP archive by name.
fn read_zip_entry<R: Read + Seek>(
    archive: &mut zip::ZipArchive<R>,
    name: &str,
) -> Result<Vec<u8>, EpubError> {
    let mut file = archive.by_name(name)?;
    let size = file.size();
    if size > MAX_ENTRY_SIZE {
        return Err(EpubError::WriteError(format!(
            "ZIP entry '{}' exceeds maximum size ({} bytes > {} bytes)",
            name, size, MAX_ENTRY_SIZE
        )));
    }
    // Safe cast: size is guaranteed <= MAX_ENTRY_SIZE which fits in usize on all platforms
    let mut buf = Vec::with_capacity(size as usize);
    file.read_to_end(&mut buf)?;
    Ok(buf)
}

/// Resolve `href` against the directory `base` (e.g. "EPUB/"), normalizing
/// `..` and `.` segments. Returns a clean POSIX zip path.
///
/// Examples:
///   resolve_relative("EPUB/", "ch1.xhtml")            -> "EPUB/ch1.xhtml"
///   resolve_relative("EPUB/", "../media/img.jpg")     -> "media/img.jpg"
///   resolve_relative("EPUB/sub/", "../style.css")     -> "EPUB/style.css"
///   resolve_relative("", "OPS/content.opf")           -> "OPS/content.opf"
pub fn resolve_relative(base: &str, href: &str) -> String {
    let mut parts: Vec<&str> = base.split('/').filter(|s| !s.is_empty()).collect();
    for seg in href.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}

/// Check the mimetype file. Logs warnings but does not fail.
fn verify_mimetype<R: Read + Seek>(archive: &mut zip::ZipArchive<R>) {
    if let Ok(mut file) = archive.by_name("mimetype") {
        let mut content = String::new();
        if file.read_to_string(&mut content).is_ok() {
            let trimmed = content.trim();
            if trimmed != "application/epub+zip" {
                eprintln!(
                    "Warning: mimetype file contains '{}', expected 'application/epub+zip'",
                    trimmed
                );
            }
        }
    }
}
