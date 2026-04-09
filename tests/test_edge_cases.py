"""Tests for graceful handling of malformed EPUBs."""

from pathlib import Path

import pytest
from fast_ebook import epub

FIXTURES = Path(__file__).parent / "fixtures"


def test_missing_metadata_parses():
    """EPUB with no <metadata> should still parse (empty metadata)."""
    book = epub.read_epub(str(FIXTURES / "missing_metadata.epub"))
    assert book.get_metadata("DC", "title") == []
    assert len(book.get_items()) >= 1  # chapter is still there


def test_empty_manifest_parses():
    """EPUB with empty <manifest/> should parse with zero items."""
    book = epub.read_epub(str(FIXTURES / "empty_manifest.epub"))
    assert len(book.get_items()) == 0
    assert book.get_metadata("DC", "title")[0][0] == "Empty Manifest"


def test_wrong_mimetype_parses():
    """EPUB with wrong mimetype should still parse (lenient)."""
    book = epub.read_epub(str(FIXTURES / "wrong_mimetype.epub"))
    assert book.get_metadata("DC", "title")[0][0] == "Wrong Mimetype"
    assert len(book.get_items()) >= 1


def test_no_spine_parses():
    """EPUB missing <spine> should parse with empty spine."""
    book = epub.read_epub(str(FIXTURES / "no_spine.epub"))
    assert book.get_metadata("DC", "title")[0][0] == "No Spine"
    assert len(book.get_spine()) == 0
    assert len(book.get_items()) >= 1


def test_missing_container_raises():
    """EPUB without META-INF/container.xml should raise."""
    import zipfile, tempfile
    with tempfile.NamedTemporaryFile(suffix=".epub", delete=False) as f:
        with zipfile.ZipFile(f, "w") as zf:
            info = zipfile.ZipInfo("mimetype")
            info.compress_type = zipfile.ZIP_STORED
            zf.writestr(info, "application/epub+zip")
        path = f.name

    with pytest.raises(ValueError, match="container"):
        epub.read_epub(path)

    import os
    os.unlink(path)


def test_non_zip_file_raises(tmp_path):
    """A non-ZIP file should raise a clear error."""
    bad = tmp_path / "bad.epub"
    bad.write_text("this is not a zip")
    with pytest.raises(ValueError):
        epub.read_epub(str(bad))
