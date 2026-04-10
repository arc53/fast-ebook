"""
Validate EPUBs produced by fast-ebook using W3C EPUBCheck.

Requires: Java runtime + epubcheck.jar (downloaded in CI, or set EPUBCHECK_JAR env var).
Skipped if EPUBCheck is not available.
"""

import os
import subprocess
import shutil
import tempfile

import pytest
from fast_ebook import epub, ITEM_DOCUMENT

EPUBCHECK_JAR = os.environ.get("EPUBCHECK_JAR", "epubcheck.jar")


def epubcheck_available():
    """Check if Java and EPUBCheck jar are available."""
    if not shutil.which("java"):
        return False
    return os.path.isfile(EPUBCHECK_JAR)


def run_epubcheck(epub_path):
    """Run EPUBCheck on an EPUB file. Returns (success, output)."""
    result = subprocess.run(
        ["java", "-jar", EPUBCHECK_JAR, epub_path],
        capture_output=True,
        text=True,
        timeout=30,
    )
    return result.returncode == 0, result.stdout + result.stderr


pytestmark = pytest.mark.skipif(
    not epubcheck_available(),
    reason="EPUBCheck not available (set EPUBCHECK_JAR env var)",
)


class TestEpubCheckMinimal:
    def test_minimal_epub(self, tmp_path):
        book = epub.EpubBook()
        book.set_identifier("epubcheck-minimal")
        book.set_title("Minimal EPUBCheck Test")
        book.set_language("en")
        book.add_author("Test Author")

        c1 = epub.EpubHtml(title="Chapter 1", file_name="chap_01.xhtml")
        c1.content = "<h1>Hello</h1><p>World</p>"
        book.add_item(c1)
        book.add_item(epub.EpubNcx())
        book.add_item(epub.EpubNav())

        book.toc = [epub.Link("chap_01.xhtml", "Chapter 1", "ch1")]
        book.spine = ["nav", c1]

        out = tmp_path / "minimal.epub"
        epub.write_epub(str(out), book)

        ok, output = run_epubcheck(str(out))
        assert ok, f"EPUBCheck failed:\n{output}"


class TestEpubCheckMultiChapter:
    def test_multi_chapter(self, tmp_path):
        book = epub.EpubBook()
        book.set_identifier("epubcheck-multi")
        book.set_title("Multi Chapter EPUBCheck")
        book.set_language("en")
        book.add_author("Author One")
        book.add_author("Author Two")
        book.add_metadata("DC", "description", "A test book")
        book.add_metadata("DC", "publisher", "Test Press")

        chapters = []
        for i in range(1, 6):
            c = epub.EpubHtml(title=f"Chapter {i}", file_name=f"ch{i}.xhtml")
            c.content = f"<h1>Chapter {i}</h1><p>Content of chapter {i}.</p>"
            book.add_item(c)
            chapters.append(c)

        css = epub.EpubCss(uid="style", file_name="style.css",
                           content="body { font-family: serif; }")
        book.add_item(css)
        book.add_item(epub.EpubNcx())
        book.add_item(epub.EpubNav())

        book.toc = [epub.Link(f"ch{i}.xhtml", f"Chapter {i}", f"ch{i}") for i in range(1, 6)]
        book.spine = ["nav"] + chapters

        out = tmp_path / "multi.epub"
        epub.write_epub(str(out), book)

        ok, output = run_epubcheck(str(out))
        assert ok, f"EPUBCheck failed:\n{output}"


class TestEpubCheckCover:
    def test_with_cover(self, tmp_path):
        book = epub.EpubBook()
        book.set_identifier("epubcheck-cover")
        book.set_title("Cover Test")
        book.set_language("en")

        c1 = epub.EpubHtml(title="Ch1", file_name="ch1.xhtml")
        c1.content = "<h1>Chapter</h1><p>Text</p>"
        book.add_item(c1)

        # Create a real 1x1 JPEG using Python stdlib
        import struct
        import io as _io
        # Minimal valid JFIF JPEG: SOI + APP0 + DQT + SOF0 + DHT + SOS + image data + EOI
        # Easier: use the JPEG from the multi_chapter fixture
        from pathlib import Path
        fixture_epub = Path(__file__).parent / "fixtures" / "multi_chapter.epub"
        import zipfile
        with zipfile.ZipFile(str(fixture_epub)) as zf:
            cover_data = zf.read("OEBPS/images/cover.jpg")
        book.set_cover("cover.jpg", cover_data)
        book.add_item(epub.EpubNcx())
        book.add_item(epub.EpubNav())

        book.toc = [epub.Link("ch1.xhtml", "Chapter", "ch1")]
        book.spine = ["nav", c1]

        out = tmp_path / "cover.epub"
        epub.write_epub(str(out), book)

        ok, output = run_epubcheck(str(out))
        assert ok, f"EPUBCheck failed:\n{output}"


class TestEpubCheckNestedToc:
    def test_nested_toc(self, tmp_path):
        book = epub.EpubBook()
        book.set_identifier("epubcheck-nested")
        book.set_title("Nested ToC Test")
        book.set_language("en")

        c1 = epub.EpubHtml(title="Ch1", file_name="ch1.xhtml")
        c1.content = "<h1>Chapter 1</h1><p>Text</p>"
        c2 = epub.EpubHtml(title="Ch2", file_name="ch2.xhtml")
        c2.content = "<h1>Chapter 2</h1><p>Text</p>"
        c3 = epub.EpubHtml(title="Ch3", file_name="ch3.xhtml")
        c3.content = "<h1>Chapter 3</h1><p>Text</p>"
        book.add_item(c1)
        book.add_item(c2)
        book.add_item(c3)
        book.add_item(epub.EpubNcx())
        book.add_item(epub.EpubNav())

        book.toc = [
            (epub.Section("Part 1"), [
                epub.Link("ch1.xhtml", "Chapter 1", "ch1"),
                epub.Link("ch2.xhtml", "Chapter 2", "ch2"),
            ]),
            (epub.Section("Part 2"), [
                epub.Link("ch3.xhtml", "Chapter 3", "ch3"),
            ]),
        ]
        book.spine = [c1, c2, c3]

        out = tmp_path / "nested.epub"
        epub.write_epub(str(out), book)

        ok, output = run_epubcheck(str(out))
        assert ok, f"EPUBCheck failed:\n{output}"


class TestEpubCheckRoundtrip:
    def test_roundtrip_fixture(self, epub3_path, tmp_path):
        """Read a fixture, write it out, validate with EPUBCheck."""
        book = epub.read_epub(epub3_path)
        out = tmp_path / "roundtrip.epub"
        epub.write_epub(str(out), book)

        ok, output = run_epubcheck(str(out))
        assert ok, f"EPUBCheck failed:\n{output}"


# Parametrized roundtrip across every well-formed fixture in tests/fixtures/.
# Skipped automatically (alongside the rest of this module) when EPUBCheck
# isn't installed. Negative fixtures (missing metadata, no spine, etc.) are
# excluded since they intentionally fail validation.
from pathlib import Path  # noqa: E402

_FIXTURES = Path(__file__).parent / "fixtures"
_VALID_FIXTURES = [
    _FIXTURES / "minimal_epub2.epub",
    _FIXTURES / "minimal_epub3.epub",
    _FIXTURES / "multi_chapter.epub",
    _FIXTURES / "nested_toc.epub",
]


@pytest.mark.parametrize(
    "fixture", _VALID_FIXTURES, ids=lambda p: p.stem
)
def test_roundtrip_all_fixtures(fixture, tmp_path):
    """Roundtrip every valid fixture and validate the output with EPUBCheck."""
    book = epub.read_epub(str(fixture))
    out = tmp_path / f"{fixture.stem}_roundtrip.epub"
    epub.write_epub(str(out), book)

    ok, output = run_epubcheck(str(out))
    assert ok, f"EPUBCheck failed for {fixture.name}:\n{output}"
