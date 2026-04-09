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
        let zip_path = if opf_base.is_empty() {
            manifest_item.href.clone()
        } else {
            format!("{}{}", opf_base, manifest_item.href)
        };

        let item_type = ItemType::from_media_type(
            &manifest_item.media_type,
            manifest_item.properties.as_deref(),
        );

        let item = if let Some(ref zd) = zip_data {
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
            let nav_path = if opf_base.is_empty() {
                nav_manifest.href.clone()
            } else {
                format!("{}{}", opf_base, nav_manifest.href)
            };
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
                let ncx_path = if opf_base.is_empty() {
                    ncx_manifest.href.clone()
                } else {
                    format!("{}{}", opf_base, ncx_manifest.href)
                };
                if let Ok(ncx_content) = read_zip_entry(archive, &ncx_path) {
                    let ncx_str = String::from_utf8_lossy(&ncx_content);
                    if let Ok(ncx_toc) = ncx::parse_ncx(&ncx_str) {
                        toc = ncx_toc;
                    }
                }
            }
        }
    }

    // 8. Build and return EpubBook
    Ok(EpubBook::new(meta, items, spine_items, toc))
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
