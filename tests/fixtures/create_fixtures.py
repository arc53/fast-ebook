"""Generate test EPUB fixtures using only stdlib zipfile."""

import zipfile
from pathlib import Path

FIXTURES_DIR = Path(__file__).parent


def write_mimetype(zf):
    """Write the mimetype file as the first uncompressed entry."""
    info = zipfile.ZipInfo("mimetype")
    info.compress_type = zipfile.ZIP_STORED
    zf.writestr(info, "application/epub+zip")


CONTAINER_XML = """\
<?xml version="1.0" encoding="UTF-8"?>
<container xmlns="urn:oasis:names:tc:opendocument:xmlns:container" version="1.0">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"""


def create_minimal_epub2():
    """Create a minimal valid EPUB2 with one chapter and NCX."""
    opf = """\
<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" unique-identifier="uid" version="2.0">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:opf="http://www.idpf.org/2007/opf">
    <dc:title>Minimal EPUB2</dc:title>
    <dc:language>en</dc:language>
    <dc:identifier id="uid">test-epub2-001</dc:identifier>
    <dc:creator opf:role="aut" opf:file-as="Author, Test">Test Author</dc:creator>
  </metadata>
  <manifest>
    <item id="ncx" href="toc.ncx" media-type="application/x-dtbncx+xml"/>
    <item id="ch1" href="chapter1.xhtml" media-type="application/xhtml+xml"/>
  </manifest>
  <spine toc="ncx">
    <itemref idref="ch1"/>
  </spine>
</package>"""

    ncx = """\
<?xml version="1.0" encoding="UTF-8"?>
<ncx xmlns="http://www.daisy.org/z3986/2005/ncx/" version="2005-1">
  <head>
    <meta name="dtb:uid" content="test-epub2-001"/>
  </head>
  <docTitle><text>Minimal EPUB2</text></docTitle>
  <navMap>
    <navPoint id="np1" playOrder="1">
      <navLabel><text>Chapter 1</text></navLabel>
      <content src="chapter1.xhtml"/>
    </navPoint>
  </navMap>
</ncx>"""

    chapter1 = """\
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml">
<head><title>Chapter 1</title></head>
<body>
<h1>Chapter 1</h1>
<p>This is the first chapter of the minimal EPUB2 test book.</p>
</body>
</html>"""

    path = FIXTURES_DIR / "minimal_epub2.epub"
    with zipfile.ZipFile(path, "w") as zf:
        write_mimetype(zf)
        zf.writestr("META-INF/container.xml", CONTAINER_XML, compress_type=zipfile.ZIP_DEFLATED)
        zf.writestr("OEBPS/content.opf", opf, compress_type=zipfile.ZIP_DEFLATED)
        zf.writestr("OEBPS/toc.ncx", ncx, compress_type=zipfile.ZIP_DEFLATED)
        zf.writestr("OEBPS/chapter1.xhtml", chapter1, compress_type=zipfile.ZIP_DEFLATED)
    print(f"Created {path}")


def create_minimal_epub3():
    """Create a minimal valid EPUB3 with one chapter and Nav document."""
    opf = """\
<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" unique-identifier="uid" version="3.0">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Minimal EPUB3</dc:title>
    <dc:language>en</dc:language>
    <dc:identifier id="uid">test-epub3-001</dc:identifier>
    <dc:creator>Test Author</dc:creator>
    <meta property="dcterms:modified">2024-01-01T00:00:00Z</meta>
  </metadata>
  <manifest>
    <item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>
    <item id="ch1" href="chapter1.xhtml" media-type="application/xhtml+xml"/>
  </manifest>
  <spine>
    <itemref idref="ch1"/>
  </spine>
</package>"""

    nav_doc = """\
<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops">
<head><title>Navigation</title></head>
<body>
  <nav epub:type="toc">
    <h1>Table of Contents</h1>
    <ol>
      <li><a href="chapter1.xhtml">Chapter 1</a></li>
    </ol>
  </nav>
</body>
</html>"""

    chapter1 = """\
<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
<head><title>Chapter 1</title></head>
<body>
<h1>Chapter 1</h1>
<p>This is the first chapter of the minimal EPUB3 test book.</p>
</body>
</html>"""

    path = FIXTURES_DIR / "minimal_epub3.epub"
    with zipfile.ZipFile(path, "w") as zf:
        write_mimetype(zf)
        zf.writestr("META-INF/container.xml", CONTAINER_XML, compress_type=zipfile.ZIP_DEFLATED)
        zf.writestr("OEBPS/content.opf", opf, compress_type=zipfile.ZIP_DEFLATED)
        zf.writestr("OEBPS/nav.xhtml", nav_doc, compress_type=zipfile.ZIP_DEFLATED)
        zf.writestr("OEBPS/chapter1.xhtml", chapter1, compress_type=zipfile.ZIP_DEFLATED)
    print(f"Created {path}")


def create_multi_chapter():
    """Create an EPUB3 with multiple chapters, CSS, an image, and both NCX + Nav."""
    # Tiny 1x1 red JPEG (smallest valid JPEG)
    tiny_jpeg = bytes([
        0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01,
        0x01, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0xFF, 0xDB, 0x00, 0x43,
        0x00, 0x08, 0x06, 0x06, 0x07, 0x06, 0x05, 0x08, 0x07, 0x07, 0x07, 0x09,
        0x09, 0x08, 0x0A, 0x0C, 0x14, 0x0D, 0x0C, 0x0B, 0x0B, 0x0C, 0x19, 0x12,
        0x13, 0x0F, 0x14, 0x1D, 0x1A, 0x1F, 0x1E, 0x1D, 0x1A, 0x1C, 0x1C, 0x20,
        0x24, 0x2E, 0x27, 0x20, 0x22, 0x2C, 0x23, 0x1C, 0x1C, 0x28, 0x37, 0x29,
        0x2C, 0x30, 0x31, 0x34, 0x34, 0x34, 0x1F, 0x27, 0x39, 0x3D, 0x38, 0x32,
        0x3C, 0x2E, 0x33, 0x34, 0x32, 0xFF, 0xC0, 0x00, 0x0B, 0x08, 0x00, 0x01,
        0x00, 0x01, 0x01, 0x01, 0x11, 0x00, 0xFF, 0xC4, 0x00, 0x1F, 0x00, 0x00,
        0x01, 0x05, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
        0x09, 0x0A, 0x0B, 0xFF, 0xC4, 0x00, 0xB5, 0x10, 0x00, 0x02, 0x01, 0x03,
        0x03, 0x02, 0x04, 0x03, 0x05, 0x05, 0x04, 0x04, 0x00, 0x00, 0x01, 0x7D,
        0x01, 0x02, 0x03, 0x00, 0x04, 0x11, 0x05, 0x12, 0x21, 0x31, 0x41, 0x06,
        0x13, 0x51, 0x61, 0x07, 0x22, 0x71, 0x14, 0x32, 0x81, 0x91, 0xA1, 0x08,
        0x23, 0x42, 0xB1, 0xC1, 0x15, 0x52, 0xD1, 0xF0, 0x24, 0x33, 0x62, 0x72,
        0x82, 0x09, 0x0A, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x25, 0x26, 0x27, 0x28,
        0x29, 0x2A, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3A, 0x43, 0x44, 0x45,
        0x46, 0x47, 0x48, 0x49, 0x4A, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59,
        0x5A, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6A, 0x73, 0x74, 0x75,
        0x76, 0x77, 0x78, 0x79, 0x7A, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89,
        0x8A, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9A, 0xA2, 0xA3,
        0xA4, 0xA5, 0xA6, 0xA7, 0xA8, 0xA9, 0xAA, 0xB2, 0xB3, 0xB4, 0xB5, 0xB6,
        0xB7, 0xB8, 0xB9, 0xBA, 0xC2, 0xC3, 0xC4, 0xC5, 0xC6, 0xC7, 0xC8, 0xC9,
        0xCA, 0xD2, 0xD3, 0xD4, 0xD5, 0xD6, 0xD7, 0xD8, 0xD9, 0xDA, 0xE1, 0xE2,
        0xE3, 0xE4, 0xE5, 0xE6, 0xE7, 0xE8, 0xE9, 0xEA, 0xF1, 0xF2, 0xF3, 0xF4,
        0xF5, 0xF6, 0xF7, 0xF8, 0xF9, 0xFA, 0xFF, 0xDA, 0x00, 0x08, 0x01, 0x01,
        0x00, 0x00, 0x3F, 0x00, 0x7B, 0x94, 0x11, 0x00, 0x00, 0x00, 0x00, 0xFF,
        0xD9,
    ])

    opf = """\
<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" unique-identifier="uid" version="3.0">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:opf="http://www.idpf.org/2007/opf">
    <dc:title>Multi Chapter Book</dc:title>
    <dc:language>en</dc:language>
    <dc:identifier id="uid">test-multi-001</dc:identifier>
    <dc:creator>Author One</dc:creator>
    <dc:creator>Author Two</dc:creator>
    <dc:publisher>Test Publisher</dc:publisher>
    <dc:date>2024-01-15</dc:date>
    <dc:description>A test book with multiple chapters.</dc:description>
    <meta property="dcterms:modified">2024-01-15T00:00:00Z</meta>
  </metadata>
  <manifest>
    <item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>
    <item id="ncx" href="toc.ncx" media-type="application/x-dtbncx+xml"/>
    <item id="ch1" href="chapter1.xhtml" media-type="application/xhtml+xml"/>
    <item id="ch2" href="chapter2.xhtml" media-type="application/xhtml+xml"/>
    <item id="ch3" href="chapter3.xhtml" media-type="application/xhtml+xml"/>
    <item id="css" href="style.css" media-type="text/css"/>
    <item id="cover-img" href="images/cover.jpg" media-type="image/jpeg" properties="cover-image"/>
  </manifest>
  <spine toc="ncx">
    <itemref idref="ch1"/>
    <itemref idref="ch2"/>
    <itemref idref="ch3"/>
  </spine>
</package>"""

    nav_doc = """\
<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops">
<head><title>Navigation</title></head>
<body>
  <nav epub:type="toc">
    <h1>Table of Contents</h1>
    <ol>
      <li><a href="chapter1.xhtml">Introduction</a></li>
      <li><a href="chapter2.xhtml">The Middle</a></li>
      <li><a href="chapter3.xhtml">Conclusion</a></li>
    </ol>
  </nav>
</body>
</html>"""

    ncx = """\
<?xml version="1.0" encoding="UTF-8"?>
<ncx xmlns="http://www.daisy.org/z3986/2005/ncx/" version="2005-1">
  <head><meta name="dtb:uid" content="test-multi-001"/></head>
  <docTitle><text>Multi Chapter Book</text></docTitle>
  <navMap>
    <navPoint id="np1" playOrder="1">
      <navLabel><text>Introduction</text></navLabel>
      <content src="chapter1.xhtml"/>
    </navPoint>
    <navPoint id="np2" playOrder="2">
      <navLabel><text>The Middle</text></navLabel>
      <content src="chapter2.xhtml"/>
    </navPoint>
    <navPoint id="np3" playOrder="3">
      <navLabel><text>Conclusion</text></navLabel>
      <content src="chapter3.xhtml"/>
    </navPoint>
  </navMap>
</ncx>"""

    css = "body { font-family: serif; margin: 1em; }\nh1 { color: #333; }\n"

    chapters = []
    for i, title in enumerate(["Introduction", "The Middle", "Conclusion"], 1):
        chapters.append(f"""\
<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
<head><title>{title}</title><link rel="stylesheet" href="style.css"/></head>
<body>
<h1>{title}</h1>
<p>Content of chapter {i}.</p>
</body>
</html>""")

    path = FIXTURES_DIR / "multi_chapter.epub"
    with zipfile.ZipFile(path, "w") as zf:
        write_mimetype(zf)
        zf.writestr("META-INF/container.xml", CONTAINER_XML, compress_type=zipfile.ZIP_DEFLATED)
        zf.writestr("OEBPS/content.opf", opf, compress_type=zipfile.ZIP_DEFLATED)
        zf.writestr("OEBPS/nav.xhtml", nav_doc, compress_type=zipfile.ZIP_DEFLATED)
        zf.writestr("OEBPS/toc.ncx", ncx, compress_type=zipfile.ZIP_DEFLATED)
        zf.writestr("OEBPS/style.css", css, compress_type=zipfile.ZIP_DEFLATED)
        zf.writestr("OEBPS/images/cover.jpg", tiny_jpeg, compress_type=zipfile.ZIP_DEFLATED)
        for i, ch in enumerate(chapters, 1):
            zf.writestr(f"OEBPS/chapter{i}.xhtml", ch, compress_type=zipfile.ZIP_DEFLATED)
    print(f"Created {path}")


def create_nested_toc():
    """Create an EPUB3 with a nested (2-level) table of contents."""
    opf = """\
<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" unique-identifier="uid" version="3.0">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Nested ToC Book</dc:title>
    <dc:language>en</dc:language>
    <dc:identifier id="uid">test-nested-001</dc:identifier>
    <dc:creator>Test Author</dc:creator>
    <meta property="dcterms:modified">2024-01-01T00:00:00Z</meta>
  </metadata>
  <manifest>
    <item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>
    <item id="p1" href="part1.xhtml" media-type="application/xhtml+xml"/>
    <item id="ch1" href="chapter1.xhtml" media-type="application/xhtml+xml"/>
    <item id="ch2" href="chapter2.xhtml" media-type="application/xhtml+xml"/>
    <item id="p2" href="part2.xhtml" media-type="application/xhtml+xml"/>
    <item id="ch3" href="chapter3.xhtml" media-type="application/xhtml+xml"/>
  </manifest>
  <spine>
    <itemref idref="p1"/>
    <itemref idref="ch1"/>
    <itemref idref="ch2"/>
    <itemref idref="p2"/>
    <itemref idref="ch3"/>
  </spine>
</package>"""

    nav_doc = """\
<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops">
<head><title>Navigation</title></head>
<body>
  <nav epub:type="toc">
    <h1>Table of Contents</h1>
    <ol>
      <li>
        <a href="part1.xhtml">Part 1: Beginning</a>
        <ol>
          <li><a href="chapter1.xhtml">Chapter 1</a></li>
          <li><a href="chapter2.xhtml">Chapter 2</a></li>
        </ol>
      </li>
      <li>
        <a href="part2.xhtml">Part 2: End</a>
        <ol>
          <li><a href="chapter3.xhtml">Chapter 3</a></li>
        </ol>
      </li>
    </ol>
  </nav>
</body>
</html>"""

    def make_page(title, body_text):
        return f"""\
<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
<head><title>{title}</title></head>
<body>
<h1>{title}</h1>
<p>{body_text}</p>
</body>
</html>"""

    path = FIXTURES_DIR / "nested_toc.epub"
    with zipfile.ZipFile(path, "w") as zf:
        write_mimetype(zf)
        zf.writestr("META-INF/container.xml", CONTAINER_XML, compress_type=zipfile.ZIP_DEFLATED)
        zf.writestr("OEBPS/content.opf", opf, compress_type=zipfile.ZIP_DEFLATED)
        zf.writestr("OEBPS/nav.xhtml", nav_doc, compress_type=zipfile.ZIP_DEFLATED)
        zf.writestr("OEBPS/part1.xhtml", make_page("Part 1: Beginning", "The start."), compress_type=zipfile.ZIP_DEFLATED)
        zf.writestr("OEBPS/chapter1.xhtml", make_page("Chapter 1", "First chapter."), compress_type=zipfile.ZIP_DEFLATED)
        zf.writestr("OEBPS/chapter2.xhtml", make_page("Chapter 2", "Second chapter."), compress_type=zipfile.ZIP_DEFLATED)
        zf.writestr("OEBPS/part2.xhtml", make_page("Part 2: End", "The end."), compress_type=zipfile.ZIP_DEFLATED)
        zf.writestr("OEBPS/chapter3.xhtml", make_page("Chapter 3", "Third chapter."), compress_type=zipfile.ZIP_DEFLATED)
    print(f"Created {path}")


if __name__ == "__main__":
    create_minimal_epub2()
    create_minimal_epub3()
    create_multi_chapter()
    create_nested_toc()
    print("All fixtures created.")
