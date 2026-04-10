use std::fmt::Write as FmtWrite;
use std::io::{Seek, Write};

use zip::write::SimpleFileOptions;
use zip::CompressionMethod;

use crate::errors::EpubError;
use crate::item_type::ItemType;
use crate::metadata::MetadataItem;
use crate::model::EpubBook;
use crate::reader::resolve_relative;

const CONTAINER_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<container xmlns="urn:oasis:names:tc:opendocument:xmlns:container" version="1.0">
  <rootfiles>
    <rootfile full-path="EPUB/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#;

/// Write an EpubBook to an EPUB file at the given path.
pub fn write_epub(path: &str, book: &EpubBook) -> Result<(), EpubError> {
    let file = std::fs::File::create(path)?;
    write_epub_inner(file, book)?;
    Ok(())
}

/// Write an EpubBook to bytes in memory.
pub fn write_epub_to_bytes(book: &EpubBook) -> Result<Vec<u8>, EpubError> {
    let cursor = std::io::Cursor::new(Vec::new());
    write_epub_inner(cursor, book).map(|c| c.into_inner())
}

/// Core write logic, generic over any Write+Seek target.
fn write_epub_inner<W: Write + Seek>(writer: W, book: &EpubBook) -> Result<W, EpubError> {
    validate_for_write(book)?;

    let mut zip = zip::ZipWriter::new(writer);

    let opts_stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    let opts_deflated =
        SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    // 1. mimetype — MUST be first, stored, uncompressed
    zip.start_file("mimetype", opts_stored)?;
    zip.write_all(b"application/epub+zip")?;

    // 2. META-INF/container.xml
    zip.start_file("META-INF/container.xml", opts_deflated)?;
    zip.write_all(CONTAINER_XML.as_bytes())?;

    // 3. OPF
    let has_ncx = book
        .items
        .iter()
        .any(|i| i.media_type == "application/x-dtbncx+xml");
    // EPUB3 requires exactly one manifest item with the `nav` property. If
    // the source book is being written as EPUB3 and has no nav item (e.g.
    // an EPUB2 input that had only NCX, or a Python builder that forgot to
    // add EpubNav()), synthesize one from book.toc so the output remains
    // EPUB3-conformant. EPUB2 outputs intentionally skip this so they
    // round-trip losslessly.
    let writing_epub3 = !book.version.starts_with('2');
    let synth_nav = writing_epub3
        && !book.items.iter().any(|i| {
            i.item_type == ItemType::Navigation && i.media_type == "application/xhtml+xml"
        });
    let opf = generate_opf(book, has_ncx, synth_nav);
    zip.start_file("EPUB/content.opf", opts_deflated)?;
    zip.write_all(opf.as_bytes())?;

    // 4. Items. Nav and NCX content is auto-generated from `book.toc` *only*
    // when the source item has empty content (the Python builder sentinels
    // EpubNav() / EpubNcx()). On read+write roundtrip the original bytes are
    // preserved verbatim, which is required for EPUBCheck to accept the
    // result — the auto-generated nav can drop properties like image-only
    // anchors that the spec considers valid.
    let identifier = book.get_metadata_value("DC", "identifier").unwrap_or("");
    let mut generated_nav: Option<Vec<u8>> = None;
    let mut generated_ncx: Option<Vec<u8>> = None;
    for item in &book.items {
        // Resolve item href against the OPF directory ("EPUB/"), normalizing
        // `..` segments so items hosted outside the OPF directory (e.g.
        // "../media/img.jpg") land at their canonical zip location.
        let zip_path = resolve_relative("EPUB/", &item.href);
        let content = item.get_content();

        let bytes: &[u8] = if content.is_empty() && item.media_type == "application/x-dtbncx+xml" {
            let ncx =
                generated_ncx.get_or_insert_with(|| generate_ncx(book, identifier).into_bytes());
            ncx.as_slice()
        } else if content.is_empty()
            && item.item_type == ItemType::Navigation
            && item.media_type == "application/xhtml+xml"
        {
            let nav = generated_nav.get_or_insert_with(|| generate_nav(book).into_bytes());
            nav.as_slice()
        } else {
            content
        };

        zip.start_file(zip_path, opts_deflated)?;
        zip.write_all(bytes)?;
    }

    // Synthetic Nav doc (only when the source book had no nav item).
    if synth_nav {
        let nav = generate_nav(book);
        zip.start_file("EPUB/nav.xhtml", opts_deflated)?;
        zip.write_all(nav.as_bytes())?;
    }

    let writer = zip.finish()?;
    Ok(writer)
}

/// Validate that the book has all required fields for writing.
fn validate_for_write(book: &EpubBook) -> Result<(), EpubError> {
    if book.get_metadata_value("DC", "identifier").is_none() {
        return Err(EpubError::MissingIdentifier);
    }
    if book.get_metadata_value("DC", "title").is_none() {
        return Err(EpubError::MissingTitle);
    }
    if book.get_metadata_value("DC", "language").is_none() {
        return Err(EpubError::MissingLanguage);
    }
    if book.spine.is_empty() {
        return Err(EpubError::EmptySpine);
    }
    Ok(())
}

/// Generate the OPF package document XML.
fn generate_opf(book: &EpubBook, has_ncx: bool, synth_nav: bool) -> String {
    // Estimate capacity: ~120 bytes per item + metadata overhead
    let mut opf = String::with_capacity(512 + book.items.len() * 120);
    let writing_epub3 = !book.version.starts_with('2');
    let pkg_version = if writing_epub3 { "3.0" } else { "2.0" };

    // The unique-identifier on <package> must reference the id attribute on
    // the corresponding <dc:identifier>. Reuse the captured id if present so
    // metadata `<meta refines="#that-id">` refinements still resolve;
    // otherwise fall back to a synthesized "BookId".
    let identifier_id = book
        .metadata
        .get("DC")
        .and_then(|dc| dc.get("identifier"))
        .and_then(|ids| ids.first())
        .and_then(|item| item.attributes.get("id").cloned())
        .filter(|s| is_safe_xml_name(s))
        .unwrap_or_else(|| "BookId".to_string());

    opf.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    opf.push_str("<package xmlns=\"http://www.idpf.org/2007/opf\" version=\"");
    opf.push_str(pkg_version);
    opf.push_str("\" unique-identifier=\"");
    xml_escape_into(&mut opf, &identifier_id);
    opf.push_str("\">\n  <metadata xmlns:dc=\"http://purl.org/dc/elements/1.1/\" xmlns:opf=\"http://www.idpf.org/2007/opf\">\n");

    // DC metadata. Captured attributes are echoed verbatim, with two
    // EPUB3-specific transforms: opf:role / opf:file-as / opf:event /
    // opf:scheme attributes (illegal on EPUB3 dc:* elements) are stripped
    // and emitted as `<meta refines="#dc-id" property="...">value</meta>`
    // refinements instead, generating an id for the dc element if needed.
    let mut pending_refines: Vec<(String, String, String)> = Vec::new();
    let mut id_counter: u32 = 0;
    if let Some(dc) = book.metadata.get("DC") {
        if let Some(ids) = dc.get("identifier") {
            for (i, item) in ids.iter().enumerate() {
                let id = if i == 0 {
                    Some(identifier_id.as_str())
                } else {
                    item.attributes.get("id").map(|s| s.as_str())
                };
                write_dc_element(
                    &mut opf,
                    "identifier",
                    item,
                    id,
                    writing_epub3,
                    &mut pending_refines,
                    &mut id_counter,
                );
            }
        }
        if let Some(titles) = dc.get("title") {
            for item in titles {
                write_dc_element(
                    &mut opf,
                    "title",
                    item,
                    None,
                    writing_epub3,
                    &mut pending_refines,
                    &mut id_counter,
                );
            }
        }
        if let Some(langs) = dc.get("language") {
            for item in langs {
                write_dc_element(
                    &mut opf,
                    "language",
                    item,
                    None,
                    writing_epub3,
                    &mut pending_refines,
                    &mut id_counter,
                );
            }
        }
        if let Some(creators) = dc.get("creator") {
            for item in creators {
                write_dc_element(
                    &mut opf,
                    "creator",
                    item,
                    None,
                    writing_epub3,
                    &mut pending_refines,
                    &mut id_counter,
                );
            }
        }
        for (field, items) in dc {
            if matches!(
                field.as_str(),
                "identifier" | "title" | "language" | "creator"
            ) {
                continue;
            }
            if !is_safe_xml_name(field) {
                continue;
            }
            for item in items {
                write_dc_element(
                    &mut opf,
                    field,
                    item,
                    None,
                    writing_epub3,
                    &mut pending_refines,
                    &mut id_counter,
                );
            }
        }
    }

    // EPUB3 refines metadata synthesized from opf:* attrs on DC elements.
    for (target_id, property, value) in &pending_refines {
        opf.push_str("    <meta refines=\"#");
        xml_escape_into(&mut opf, target_id);
        opf.push_str("\" property=\"");
        xml_escape_into(&mut opf, property);
        opf.push_str("\">");
        xml_escape_into(&mut opf, value);
        opf.push_str("</meta>\n");
    }

    // dcterms:modified is required by EPUB3 only — skip when writing EPUB2.
    if writing_epub3 {
        let has_modified = book
            .metadata
            .get("OPF")
            .and_then(|m| m.get("dcterms:modified"))
            .is_some_and(|v| !v.is_empty());
        if !has_modified {
            opf.push_str("    <meta property=\"dcterms:modified\">");
            opf.push_str(&current_utc_timestamp());
            opf.push_str("</meta>\n");
        }
    }

    // OPF metadata. We emit all attributes captured at parse time so that
    // EPUB3 refinements (`refines`, `id`, `scheme`, etc.) survive round-trip
    // — EPUBCheck enforces these for things like `media:duration`,
    // `dcterms:modified`, and DCMES element refinements.
    if let Some(opf_meta) = book.metadata.get("OPF") {
        for (name, items) in opf_meta {
            if !is_safe_xml_name(name) {
                continue; // skip meta entries with unsafe names
            }
            for item in items {
                // Pick the serialization form from the captured attributes:
                // EPUB3 metas have a `property` attribute, EPUB2 metas have
                // `name` + `content`. The hash key may not contain ':' even
                // for EPUB3 (e.g. property="belongs-to-collection").
                if item.attributes.contains_key("property") {
                    write_epub3_meta(&mut opf, name, item);
                } else if item.attributes.contains_key("name")
                    || item.attributes.contains_key("content")
                {
                    write_epub2_meta(&mut opf, name, item);
                } else if name.contains(':') {
                    // Fall back: synthesized via Python add_metadata("OPF", "ns:foo", v)
                    write_epub3_meta(&mut opf, name, item);
                } else {
                    write_epub2_meta(&mut opf, name, item);
                }
            }
        }
    }

    opf.push_str("  </metadata>\n  <manifest>\n");

    // Manifest. Properties / media-overlay / fallback are emitted verbatim
    // from the source item if present (required for EPUBCheck round-trip);
    // otherwise we synthesize the minimum needed for items built via the
    // Python builder API (cover-image, nav).
    for item in &book.items {
        opf.push_str("    <item id=\"");
        xml_escape_into(&mut opf, &item.id);
        opf.push_str("\" href=\"");
        xml_escape_into(&mut opf, &item.href);
        opf.push_str("\" media-type=\"");
        xml_escape_into(&mut opf, &item.media_type);
        opf.push('"');

        let synthesized_properties = match item.item_type {
            ItemType::Cover => Some("cover-image"),
            ItemType::Navigation if item.media_type == "application/xhtml+xml" => Some("nav"),
            _ => None,
        };
        if let Some(props) = item.properties.as_deref() {
            opf.push_str(" properties=\"");
            xml_escape_into(&mut opf, props);
            opf.push('"');
        } else if let Some(props) = synthesized_properties {
            opf.push_str(" properties=\"");
            opf.push_str(props);
            opf.push('"');
        }

        if let Some(mo) = item.media_overlay.as_deref() {
            opf.push_str(" media-overlay=\"");
            xml_escape_into(&mut opf, mo);
            opf.push('"');
        }
        if let Some(fb) = item.fallback.as_deref() {
            opf.push_str(" fallback=\"");
            xml_escape_into(&mut opf, fb);
            opf.push('"');
        }

        opf.push_str("/>\n");
    }

    if synth_nav {
        opf.push_str(
            "    <item id=\"nav\" href=\"nav.xhtml\" media-type=\"application/xhtml+xml\" properties=\"nav\"/>\n",
        );
    }

    opf.push_str("  </manifest>\n  <spine");

    // Spine
    if has_ncx {
        if let Some(ncx) = book
            .items
            .iter()
            .find(|i| i.media_type == "application/x-dtbncx+xml")
        {
            opf.push_str(" toc=\"");
            xml_escape_into(&mut opf, &ncx.id);
            opf.push('"');
        }
    }
    opf.push_str(">\n");

    for entry in &book.spine {
        opf.push_str("    <itemref idref=\"");
        xml_escape_into(&mut opf, &entry.idref);
        opf.push('"');
        if !entry.linear {
            opf.push_str(" linear=\"no\"");
        }
        if let Some(props) = entry.properties.as_deref() {
            opf.push_str(" properties=\"");
            xml_escape_into(&mut opf, props);
            opf.push('"');
        }
        opf.push_str("/>\n");
    }

    opf.push_str("  </spine>\n</package>\n");
    opf
}

/// Generate NCX document from the book's table of contents.
fn generate_ncx(book: &EpubBook, identifier: &str) -> String {
    let title = book.get_metadata_value("DC", "title").unwrap_or("Untitled");
    let mut ncx = String::with_capacity(256 + book.toc.len() * 150);
    ncx.push_str(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
        <ncx xmlns=\"http://www.daisy.org/z3986/2005/ncx/\" version=\"2005-1\">\n\
        \x20\x20<head>\n    <meta name=\"dtb:uid\" content=\"",
    );
    xml_escape_into(&mut ncx, identifier);
    ncx.push_str("\"/>\n  </head>\n  <docTitle><text>");
    xml_escape_into(&mut ncx, title);
    ncx.push_str("</text></docTitle>\n  <navMap>\n");

    let mut play_order = 1;
    for entry in &book.toc {
        write_ncx_navpoint(&mut ncx, entry, &mut play_order, 2);
    }

    ncx.push_str("  </navMap>\n</ncx>\n");
    ncx
}

/// Recursively write NCX navPoint elements.
fn write_ncx_navpoint(
    buf: &mut String,
    entry: &crate::ncx::TocEntry,
    play_order: &mut u32,
    indent: usize,
) {
    // If this is a section heading with no href, emit children directly
    // to avoid duplicate targets in the NCX (EPUBCheck RSC-005).
    if entry.href.is_empty() && !entry.children.is_empty() {
        for child in &entry.children {
            write_ncx_navpoint(buf, child, play_order, indent);
        }
        return;
    }

    write_indent(buf, indent);
    let _ = writeln!(
        buf,
        "<navPoint id=\"np{po}\" playOrder=\"{po}\">",
        po = play_order
    );
    write_indent(buf, indent + 1);
    buf.push_str("<navLabel><text>");
    xml_escape_into(buf, &entry.title);
    buf.push_str("</text></navLabel>\n");
    write_indent(buf, indent + 1);
    buf.push_str("<content src=\"");
    xml_escape_into(buf, &entry.href);
    buf.push_str("\"/>\n");
    *play_order += 1;

    for child in &entry.children {
        write_ncx_navpoint(buf, child, play_order, indent + 1);
    }

    write_indent(buf, indent);
    buf.push_str("</navPoint>\n");
}

/// Generate EPUB3 Nav document from the book's table of contents.
fn generate_nav(book: &EpubBook) -> String {
    let mut nav = String::with_capacity(256 + book.toc.len() * 80);
    nav.push_str(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
        <html xmlns=\"http://www.w3.org/1999/xhtml\" xmlns:epub=\"http://www.idpf.org/2007/ops\">\n\
        <head><title>Table of Contents</title></head>\n\
        <body>\n\
        \x20\x20<nav epub:type=\"toc\">\n\
        \x20\x20\x20\x20<h1>Table of Contents</h1>\n",
    );

    write_nav_ol(&mut nav, &book.toc, 2);

    nav.push_str("  </nav>\n</body>\n</html>\n");
    nav
}

/// Recursively write Nav <ol> elements.
fn write_nav_ol(buf: &mut String, entries: &[crate::ncx::TocEntry], indent: usize) {
    if entries.is_empty() {
        return;
    }
    write_indent(buf, indent);
    buf.push_str("<ol>\n");
    for entry in entries {
        if entry.children.is_empty() {
            write_indent(buf, indent + 1);
            buf.push_str("<li><a href=\"");
            xml_escape_into(buf, &entry.href);
            buf.push_str("\">");
            xml_escape_into(buf, &entry.title);
            buf.push_str("</a></li>\n");
        } else {
            write_indent(buf, indent + 1);
            buf.push_str("<li>\n");
            write_indent(buf, indent + 2);
            if entry.href.is_empty() {
                buf.push_str("<span>");
                xml_escape_into(buf, &entry.title);
                buf.push_str("</span>\n");
            } else {
                buf.push_str("<a href=\"");
                xml_escape_into(buf, &entry.href);
                buf.push_str("\">");
                xml_escape_into(buf, &entry.title);
                buf.push_str("</a>\n");
            }
            write_nav_ol(buf, &entry.children, indent + 2);
            write_indent(buf, indent + 1);
            buf.push_str("</li>\n");
        }
    }
    write_indent(buf, indent);
    buf.push_str("</ol>\n");
}

/// Write indentation (2 spaces per level) directly into buffer.
#[inline]
fn write_indent(buf: &mut String, level: usize) {
    for _ in 0..level {
        buf.push_str("  ");
    }
}

/// Whether `attr_name` is legal on a DC element under EPUB 3. EPUB2-only
/// `opf:*` refinements (role, file-as, event, scheme) are rejected by
/// EPUBCheck on the EPUB3 schema and must be converted to refines metadata.
fn is_epub3_dc_attr(name: &str) -> bool {
    matches!(name, "id" | "dir" | "xml:lang")
}

/// Emit a `<dc:{tag}>` element.
///
/// On EPUB3 output, opf:role / opf:file-as / opf:event / opf:scheme are
/// extracted from the captured attributes and queued in `pending_refines`
/// as `<meta refines="#dc-id" property="role">val</meta>` synthesizations,
/// auto-generating an id for the DC element when needed. EPUB2 output
/// emits the opf:* attributes verbatim (legal in the EPUB2 schema).
///
/// If `override_id` is Some, the `id` attribute uses that value (used for
/// the primary `<dc:identifier>` so it stays in sync with `unique-identifier`).
fn write_dc_element(
    opf: &mut String,
    tag: &str,
    item: &MetadataItem,
    override_id: Option<&str>,
    writing_epub3: bool,
    pending_refines: &mut Vec<(String, String, String)>,
    id_counter: &mut u32,
) {
    // Decide whether this element needs an id (either it had one, or we
    // need one to anchor refines metas synthesized from opf:* attrs).
    let needs_refines_id = writing_epub3
        && item.attributes.keys().any(|k| {
            matches!(
                k.as_str(),
                "opf:role" | "opf:file-as" | "opf:event" | "opf:scheme"
            )
        });
    let resolved_id: Option<String> = if let Some(id) = override_id {
        Some(id.to_string())
    } else if let Some(id) = item.attributes.get("id") {
        Some(id.clone())
    } else if needs_refines_id {
        *id_counter += 1;
        Some(format!("dc-id-{}", id_counter))
    } else {
        None
    };

    opf.push_str("    <dc:");
    opf.push_str(tag);

    let mut keys: Vec<&String> = item.attributes.keys().collect();
    keys.sort();
    for k in keys {
        if !is_safe_xml_name(k) {
            continue;
        }
        // The id attribute is always handled via resolved_id below.
        if k == "id" {
            continue;
        }
        // EPUB3 strips opf:* refinements (emitted as refines metas instead).
        if writing_epub3 && !is_epub3_dc_attr(k) {
            if let Some(prop) = k.strip_prefix("opf:") {
                if matches!(prop, "role" | "file-as" | "event" | "scheme") {
                    if let Some(target) = resolved_id.as_ref() {
                        pending_refines.push((
                            target.clone(),
                            prop.to_string(),
                            item.attributes[k].clone(),
                        ));
                    }
                }
            }
            continue;
        }
        let v = &item.attributes[k];
        opf.push(' ');
        xml_escape_into(opf, k);
        opf.push_str("=\"");
        xml_escape_into(opf, v);
        opf.push('"');
    }
    if let Some(id) = resolved_id.as_deref() {
        opf.push_str(" id=\"");
        xml_escape_into(opf, id);
        opf.push('"');
    }
    opf.push('>');
    xml_escape_into(opf, &item.value);
    opf.push_str("</dc:");
    opf.push_str(tag);
    opf.push_str(">\n");
}

/// Emit an EPUB3-style `<meta property="...">value</meta>` element, echoing
/// any captured attributes (refines, id, scheme, ...).
fn write_epub3_meta(opf: &mut String, property_name: &str, item: &MetadataItem) {
    opf.push_str("    <meta");
    let mut emitted_property = false;
    let mut keys: Vec<&String> = item.attributes.keys().collect();
    keys.sort(); // deterministic output
    for k in keys {
        if !is_safe_xml_name(k) {
            continue;
        }
        let v = &item.attributes[k];
        opf.push(' ');
        xml_escape_into(opf, k);
        opf.push_str("=\"");
        xml_escape_into(opf, v);
        opf.push('"');
        if k == "property" {
            emitted_property = true;
        }
    }
    if !emitted_property {
        opf.push_str(" property=\"");
        xml_escape_into(opf, property_name);
        opf.push('"');
    }
    opf.push('>');
    xml_escape_into(opf, &item.value);
    opf.push_str("</meta>\n");
}

/// Emit an EPUB2-style `<meta name="..." content="..."/>` element, echoing
/// any extra captured attributes.
fn write_epub2_meta(opf: &mut String, meta_name: &str, item: &MetadataItem) {
    opf.push_str("    <meta");
    let mut emitted_name = false;
    let mut emitted_content = false;
    let mut keys: Vec<&String> = item.attributes.keys().collect();
    keys.sort();
    for k in keys {
        if !is_safe_xml_name(k) {
            continue;
        }
        let v = &item.attributes[k];
        opf.push(' ');
        xml_escape_into(opf, k);
        opf.push_str("=\"");
        xml_escape_into(opf, v);
        opf.push('"');
        if k == "name" {
            emitted_name = true;
        }
        if k == "content" {
            emitted_content = true;
        }
    }
    if !emitted_name {
        opf.push_str(" name=\"");
        xml_escape_into(opf, meta_name);
        opf.push('"');
    }
    if !emitted_content {
        opf.push_str(" content=\"");
        xml_escape_into(opf, &item.value);
        opf.push('"');
    }
    opf.push_str("/>\n");
}

/// Check if a string is a safe XML element/attribute name.
/// Only allows alphanumeric, hyphens, underscores, dots, and colons (for namespaces).
#[inline]
fn is_safe_xml_name(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == ':')
}

/// Escape XML special characters directly into a buffer (zero allocations).
#[inline]
fn xml_escape_into(buf: &mut String, s: &str) {
    for ch in s.chars() {
        match ch {
            '&' => buf.push_str("&amp;"),
            '<' => buf.push_str("&lt;"),
            '>' => buf.push_str("&gt;"),
            '"' => buf.push_str("&quot;"),
            '\'' => buf.push_str("&apos;"),
            _ => buf.push(ch),
        }
    }
}

/// Allocating xml_escape (kept for test compatibility).
#[cfg(test)]
fn xml_escape(s: &str) -> String {
    let mut buf = String::with_capacity(s.len());
    xml_escape_into(&mut buf, s);
    buf
}

/// Generate current UTC timestamp in ISO 8601 format without external deps.
fn current_utc_timestamp() -> String {
    let dur = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let mut secs = dur.as_secs() as i64;

    // Days since epoch
    let mut days = secs / 86400;
    secs %= 86400;
    let hours = secs / 3600;
    secs %= 3600;
    let minutes = secs / 60;
    let seconds = secs % 60;

    // Civil date from days (algorithm from Howard Hinnant)
    days += 719468;
    let era = (if days >= 0 { days } else { days - 146096 }) / 146097;
    let doe = (days - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!("{y:04}-{m:02}-{d:02}T{hours:02}:{minutes:02}:{seconds:02}Z")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::EpubItem;
    use crate::ncx::TocEntry;
    use crate::spine::SpineItem;
    use std::sync::Arc;

    fn make_test_book() -> EpubBook {
        let mut book = EpubBook::default();
        book.set_metadata(
            "DC",
            "identifier",
            "test-id-001",
            std::collections::HashMap::new(),
        );
        book.set_metadata("DC", "title", "Test Book", std::collections::HashMap::new());
        book.set_metadata("DC", "language", "en", std::collections::HashMap::new());

        book.add_item(Arc::new(EpubItem::eager(
            "ch1".to_string(),
            "chapter1.xhtml".to_string(),
            "application/xhtml+xml".to_string(),
            ItemType::Document,
            b"<html><body><h1>Hello</h1></body></html>".to_vec(),
        )));

        book.add_item(Arc::new(EpubItem::eager(
            "ncx".to_string(),
            "toc.ncx".to_string(),
            "application/x-dtbncx+xml".to_string(),
            ItemType::Navigation,
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
            properties: None,
        }];

        book.toc = vec![TocEntry {
            title: "Chapter 1".to_string(),
            href: "chapter1.xhtml".to_string(),
            children: Vec::new(),
        }];

        book
    }

    #[test]
    fn test_generate_opf_contains_required_elements() {
        let book = make_test_book();
        let opf = generate_opf(&book, true, false);
        assert!(opf.contains("<dc:identifier"));
        assert!(opf.contains("test-id-001"));
        assert!(opf.contains("<dc:title>Test Book</dc:title>"));
        assert!(opf.contains("<dc:language>en</dc:language>"));
        assert!(opf.contains("dcterms:modified"));
        assert!(opf.contains("<manifest>"));
        assert!(opf.contains("<spine"));
        assert!(opf.contains("itemref idref=\"ch1\""));
    }

    #[test]
    fn test_generate_ncx() {
        let book = make_test_book();
        let ncx = generate_ncx(&book, "test-id-001");
        assert!(ncx.contains("<navMap>"));
        assert!(ncx.contains("Chapter 1"));
        assert!(ncx.contains("chapter1.xhtml"));
        assert!(ncx.contains("playOrder=\"1\""));
    }

    #[test]
    fn test_generate_nav() {
        let book = make_test_book();
        let nav = generate_nav(&book);
        assert!(nav.contains("epub:type=\"toc\""));
        assert!(nav.contains("Chapter 1"));
        assert!(nav.contains("chapter1.xhtml"));
    }

    #[test]
    fn test_write_epub_creates_file() {
        let book = make_test_book();
        let path = "/tmp/fast_ebook_test_write.epub";
        write_epub(path, &book).unwrap();

        // Verify it's a valid ZIP with correct structure
        let file = std::fs::File::open(path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();

        // Mimetype must be first
        let first = archive.by_index(0).unwrap();
        assert_eq!(first.name(), "mimetype");
        assert_eq!(first.compression(), CompressionMethod::Stored);
        drop(first);

        // Must contain required files
        let names: Vec<_> = (0..archive.len())
            .map(|i| archive.by_index(i).unwrap().name().to_string())
            .collect();
        assert!(names.contains(&"META-INF/container.xml".to_string()));
        assert!(names.contains(&"EPUB/content.opf".to_string()));
        assert!(names.contains(&"EPUB/toc.ncx".to_string()));
        assert!(names.contains(&"EPUB/nav.xhtml".to_string()));
        assert!(names.contains(&"EPUB/chapter1.xhtml".to_string()));

        std::fs::remove_file(path).ok();
    }

    #[test]
    fn test_validate_missing_identifier() {
        let mut book = EpubBook::default();
        book.set_metadata("DC", "title", "Test", std::collections::HashMap::new());
        book.set_metadata("DC", "language", "en", std::collections::HashMap::new());
        book.spine = vec![SpineItem {
            idref: "ch1".to_string(),
            linear: true,
            properties: None,
        }];
        assert!(matches!(
            validate_for_write(&book),
            Err(EpubError::MissingIdentifier)
        ));
    }

    #[test]
    fn test_validate_empty_spine() {
        let mut book = EpubBook::default();
        book.set_metadata("DC", "identifier", "id", std::collections::HashMap::new());
        book.set_metadata("DC", "title", "Test", std::collections::HashMap::new());
        book.set_metadata("DC", "language", "en", std::collections::HashMap::new());
        assert!(matches!(
            validate_for_write(&book),
            Err(EpubError::EmptySpine)
        ));
    }

    #[test]
    fn test_xml_escape() {
        assert_eq!(xml_escape("a<b>c&d\"e"), "a&lt;b&gt;c&amp;d&quot;e");
    }

    #[test]
    fn test_nested_toc_ncx() {
        let mut book = make_test_book();
        book.toc = vec![TocEntry {
            title: "Part 1".to_string(),
            href: "part1.xhtml".to_string(),
            children: vec![
                TocEntry {
                    title: "Ch 1".to_string(),
                    href: "ch1.xhtml".to_string(),
                    children: Vec::new(),
                },
                TocEntry {
                    title: "Ch 2".to_string(),
                    href: "ch2.xhtml".to_string(),
                    children: Vec::new(),
                },
            ],
        }];
        let ncx = generate_ncx(&book, "test-id");
        assert!(ncx.contains("Part 1"));
        assert!(ncx.contains("Ch 1"));
        assert!(ncx.contains("Ch 2"));
        assert!(ncx.contains("playOrder=\"1\""));
        assert!(ncx.contains("playOrder=\"2\""));
        assert!(ncx.contains("playOrder=\"3\""));
    }
}
