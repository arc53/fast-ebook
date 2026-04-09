"""Generate malformed EPUB fixtures for edge case testing."""

import zipfile
from pathlib import Path

FIXTURES_DIR = Path(__file__).parent


def write_mimetype(zf):
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

CHAPTER = """\
<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
<head><title>Chapter</title></head>
<body><h1>Hello</h1><p>Content</p></body>
</html>"""


def create_missing_metadata():
    """OPF with no <metadata> element."""
    opf = """\
<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="uid">
  <manifest>
    <item id="ch1" href="chapter1.xhtml" media-type="application/xhtml+xml"/>
  </manifest>
  <spine>
    <itemref idref="ch1"/>
  </spine>
</package>"""

    path = FIXTURES_DIR / "missing_metadata.epub"
    with zipfile.ZipFile(path, "w") as zf:
        write_mimetype(zf)
        zf.writestr("META-INF/container.xml", CONTAINER_XML, compress_type=zipfile.ZIP_DEFLATED)
        zf.writestr("OEBPS/content.opf", opf, compress_type=zipfile.ZIP_DEFLATED)
        zf.writestr("OEBPS/chapter1.xhtml", CHAPTER, compress_type=zipfile.ZIP_DEFLATED)
    print(f"Created {path}")


def create_empty_manifest():
    """OPF with empty <manifest/>."""
    opf = """\
<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="uid">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Empty Manifest</dc:title>
    <dc:language>en</dc:language>
    <dc:identifier id="uid">edge-empty-manifest</dc:identifier>
  </metadata>
  <manifest/>
  <spine/>
</package>"""

    path = FIXTURES_DIR / "empty_manifest.epub"
    with zipfile.ZipFile(path, "w") as zf:
        write_mimetype(zf)
        zf.writestr("META-INF/container.xml", CONTAINER_XML, compress_type=zipfile.ZIP_DEFLATED)
        zf.writestr("OEBPS/content.opf", opf, compress_type=zipfile.ZIP_DEFLATED)
    print(f"Created {path}")


def create_wrong_mimetype():
    """Mimetype file has wrong content."""
    opf = """\
<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="uid">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Wrong Mimetype</dc:title>
    <dc:language>en</dc:language>
    <dc:identifier id="uid">edge-wrong-mimetype</dc:identifier>
  </metadata>
  <manifest>
    <item id="ch1" href="chapter1.xhtml" media-type="application/xhtml+xml"/>
  </manifest>
  <spine>
    <itemref idref="ch1"/>
  </spine>
</package>"""

    path = FIXTURES_DIR / "wrong_mimetype.epub"
    with zipfile.ZipFile(path, "w") as zf:
        info = zipfile.ZipInfo("mimetype")
        info.compress_type = zipfile.ZIP_STORED
        zf.writestr(info, "text/plain")
        zf.writestr("META-INF/container.xml", CONTAINER_XML, compress_type=zipfile.ZIP_DEFLATED)
        zf.writestr("OEBPS/content.opf", opf, compress_type=zipfile.ZIP_DEFLATED)
        zf.writestr("OEBPS/chapter1.xhtml", CHAPTER, compress_type=zipfile.ZIP_DEFLATED)
    print(f"Created {path}")


def create_no_spine():
    """OPF missing <spine> element."""
    opf = """\
<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="uid">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>No Spine</dc:title>
    <dc:language>en</dc:language>
    <dc:identifier id="uid">edge-no-spine</dc:identifier>
  </metadata>
  <manifest>
    <item id="ch1" href="chapter1.xhtml" media-type="application/xhtml+xml"/>
  </manifest>
</package>"""

    path = FIXTURES_DIR / "no_spine.epub"
    with zipfile.ZipFile(path, "w") as zf:
        write_mimetype(zf)
        zf.writestr("META-INF/container.xml", CONTAINER_XML, compress_type=zipfile.ZIP_DEFLATED)
        zf.writestr("OEBPS/content.opf", opf, compress_type=zipfile.ZIP_DEFLATED)
        zf.writestr("OEBPS/chapter1.xhtml", CHAPTER, compress_type=zipfile.ZIP_DEFLATED)
    print(f"Created {path}")


if __name__ == "__main__":
    create_missing_metadata()
    create_empty_manifest()
    create_wrong_mimetype()
    create_no_spine()
    print("All edge case fixtures created.")
