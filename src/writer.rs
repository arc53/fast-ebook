use std::fmt::Write as FmtWrite;
use std::io::{Seek, Write};

use zip::write::SimpleFileOptions;
use zip::CompressionMethod;

use crate::errors::EpubError;
use crate::item_type::ItemType;
use crate::model::EpubBook;

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
    let opf = generate_opf(book, has_ncx);
    zip.start_file("EPUB/content.opf", opts_deflated)?;
    zip.write_all(opf.as_bytes())?;

    // 4. NCX (auto-generated from toc)
    if has_ncx {
        let identifier = book.get_metadata_value("DC", "identifier").unwrap_or("");
        let ncx = generate_ncx(book, identifier);
        zip.start_file("EPUB/toc.ncx", opts_deflated)?;
        zip.write_all(ncx.as_bytes())?;
    }

    // 5. Nav (auto-generated from toc)
    let has_nav = book
        .items
        .iter()
        .any(|i| i.item_type == ItemType::Navigation && i.media_type == "application/xhtml+xml");
    if has_nav {
        let nav = generate_nav(book);
        zip.start_file("EPUB/nav.xhtml", opts_deflated)?;
        zip.write_all(nav.as_bytes())?;
    }

    // 6. All other items
    for item in &book.items {
        // Skip auto-generated NCX and Nav
        if item.media_type == "application/x-dtbncx+xml" {
            continue;
        }
        if item.item_type == ItemType::Navigation && item.media_type == "application/xhtml+xml" {
            continue;
        }
        let zip_path = format!("EPUB/{}", item.href);
        zip.start_file(zip_path, opts_deflated)?;
        zip.write_all(item.get_content())?;
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
fn generate_opf(book: &EpubBook, has_ncx: bool) -> String {
    // Estimate capacity: ~120 bytes per item + metadata overhead
    let mut opf = String::with_capacity(512 + book.items.len() * 120);
    opf.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
        <package xmlns=\"http://www.idpf.org/2007/opf\" version=\"3.0\" unique-identifier=\"BookId\">\n\
        \x20\x20<metadata xmlns:dc=\"http://purl.org/dc/elements/1.1/\" xmlns:opf=\"http://www.idpf.org/2007/opf\">\n");

    // DC metadata
    if let Some(dc) = book.metadata.get("DC") {
        if let Some(ids) = dc.get("identifier") {
            for item in ids {
                opf.push_str("    <dc:identifier id=\"BookId\">");
                xml_escape_into(&mut opf, &item.value);
                opf.push_str("</dc:identifier>\n");
            }
        }
        if let Some(titles) = dc.get("title") {
            for item in titles {
                opf.push_str("    <dc:title>");
                xml_escape_into(&mut opf, &item.value);
                opf.push_str("</dc:title>\n");
            }
        }
        if let Some(langs) = dc.get("language") {
            for item in langs {
                opf.push_str("    <dc:language>");
                xml_escape_into(&mut opf, &item.value);
                opf.push_str("</dc:language>\n");
            }
        }
        if let Some(creators) = dc.get("creator") {
            for item in creators {
                opf.push_str("    <dc:creator");
                for (k, v) in &item.attributes {
                    opf.push(' ');
                    xml_escape_into(&mut opf, k);
                    opf.push_str("=\"");
                    xml_escape_into(&mut opf, v);
                    opf.push('"');
                }
                opf.push('>');
                xml_escape_into(&mut opf, &item.value);
                opf.push_str("</dc:creator>\n");
            }
        }
        for (field, items) in dc {
            if matches!(
                field.as_str(),
                "identifier" | "title" | "language" | "creator"
            ) {
                continue;
            }
            for item in items {
                if !is_safe_xml_name(field) {
                    continue; // skip fields with unsafe names
                }
                opf.push_str("    <dc:");
                opf.push_str(field);
                opf.push('>');
                xml_escape_into(&mut opf, &item.value);
                opf.push_str("</dc:");
                opf.push_str(field);
                opf.push_str(">\n");
            }
        }
    }

    // dcterms:modified (required by EPUB3) — only add if not already present
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

    // OPF metadata
    if let Some(opf_meta) = book.metadata.get("OPF") {
        for (name, items) in opf_meta {
            if !is_safe_xml_name(name) {
                continue; // skip meta entries with unsafe names
            }
            for item in items {
                if name.contains(':') {
                    opf.push_str("    <meta property=\"");
                    xml_escape_into(&mut opf, name);
                    opf.push_str("\">");
                    xml_escape_into(&mut opf, &item.value);
                    opf.push_str("</meta>\n");
                } else {
                    opf.push_str("    <meta name=\"");
                    xml_escape_into(&mut opf, name);
                    opf.push_str("\" content=\"");
                    xml_escape_into(&mut opf, &item.value);
                    opf.push_str("\"/>\n");
                }
            }
        }
    }

    opf.push_str("  </metadata>\n  <manifest>\n");

    // Manifest
    for item in &book.items {
        opf.push_str("    <item id=\"");
        xml_escape_into(&mut opf, &item.id);
        opf.push_str("\" href=\"");
        xml_escape_into(&mut opf, &item.href);
        opf.push_str("\" media-type=\"");
        xml_escape_into(&mut opf, &item.media_type);
        opf.push('"');
        match item.item_type {
            ItemType::Cover => opf.push_str(" properties=\"cover-image\""),
            ItemType::Navigation if item.media_type == "application/xhtml+xml" => {
                opf.push_str(" properties=\"nav\"")
            }
            _ => {}
        }
        opf.push_str("/>\n");
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
        let opf = generate_opf(&book, true);
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
