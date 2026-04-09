"""Comprehensive roundtrip tests: read -> write -> read -> compare."""

import io
from pathlib import Path

from fast_ebook import epub, ITEM_DOCUMENT, ITEM_IMAGE, ITEM_STYLE, ITEM_COVER, ITEM_NAVIGATION

FIXTURES = Path(__file__).parent / "fixtures"


def assert_books_equal(book1, book2, check_content=True):
    """Assert two books have equivalent metadata, items, spine, and toc."""
    # Metadata
    for field in ["title", "language", "identifier"]:
        v1 = book1.get_metadata("DC", field)
        v2 = book2.get_metadata("DC", field)
        assert [x[0] for x in v1] == [x[0] for x in v2], f"DC:{field} mismatch"

    # Item counts by type
    for item_type in [ITEM_DOCUMENT, ITEM_IMAGE, ITEM_STYLE, ITEM_COVER, ITEM_NAVIGATION]:
        c1 = len(book1.get_items_of_type(item_type))
        c2 = len(book2.get_items_of_type(item_type))
        assert c1 == c2, f"Item type {item_type} count: {c1} vs {c2}"

    # Spine order
    s1 = [idref for idref, _ in book1.get_spine()]
    s2 = [idref for idref, _ in book2.get_spine()]
    assert s1 == s2, f"Spine mismatch: {s1} vs {s2}"

    # ToC structure
    def flatten_toc(entries, depth=0):
        result = []
        for e in entries:
            result.append((e.title, e.href, depth))
            result.extend(flatten_toc(e.children, depth + 1))
        return result

    t1 = flatten_toc(book1.toc)
    t2 = flatten_toc(book2.toc)
    assert t1 == t2, f"ToC mismatch"


def test_roundtrip_epub2():
    path = str(FIXTURES / "minimal_epub2.epub")
    book1 = epub.read_epub(path)
    buf = io.BytesIO()
    epub.write_epub(buf, book1)
    book2 = epub.read_epub(buf.getvalue())
    assert_books_equal(book1, book2)


def test_roundtrip_epub3():
    path = str(FIXTURES / "minimal_epub3.epub")
    book1 = epub.read_epub(path)
    buf = io.BytesIO()
    epub.write_epub(buf, book1)
    book2 = epub.read_epub(buf.getvalue())
    assert_books_equal(book1, book2)


def test_roundtrip_multi_chapter():
    path = str(FIXTURES / "multi_chapter.epub")
    book1 = epub.read_epub(path)
    buf = io.BytesIO()
    epub.write_epub(buf, book1)
    book2 = epub.read_epub(buf.getvalue())
    assert_books_equal(book1, book2)

    # Verify cover image content
    covers1 = book1.get_items_of_type(ITEM_COVER)
    covers2 = book2.get_items_of_type(ITEM_COVER)
    if covers1 and covers2:
        assert covers1[0].get_content() == covers2[0].get_content()


def test_roundtrip_nested_toc():
    path = str(FIXTURES / "nested_toc.epub")
    book1 = epub.read_epub(path)
    buf = io.BytesIO()
    epub.write_epub(buf, book1)
    book2 = epub.read_epub(buf.getvalue())
    assert_books_equal(book1, book2)
    # Verify nesting
    assert len(book2.toc) == 2
    assert len(book2.toc[0].children) == 2


def test_roundtrip_constructed_book():
    """Build a book from scratch, write, read back, compare."""
    book1 = epub.EpubBook()
    book1.set_identifier("roundtrip-001")
    book1.set_title("Roundtrip Test")
    book1.set_language("en")
    book1.add_author("Author")

    c1 = epub.EpubHtml(title="Ch1", file_name="ch1.xhtml")
    c1.content = "<h1>Chapter 1</h1><p>Content.</p>"
    c2 = epub.EpubHtml(title="Ch2", file_name="ch2.xhtml")
    c2.content = "<h1>Chapter 2</h1><p>More content.</p>"
    book1.add_item(c1)
    book1.add_item(c2)
    book1.add_item(epub.EpubNcx())
    book1.add_item(epub.EpubNav())

    book1.toc = [
        epub.Link("ch1.xhtml", "Chapter 1", "ch1"),
        epub.Link("ch2.xhtml", "Chapter 2", "ch2"),
    ]
    book1.spine = [c1, c2]

    buf = io.BytesIO()
    epub.write_epub(buf, book1)
    book2 = epub.read_epub(buf.getvalue())

    assert book2.get_metadata("DC", "title")[0][0] == "Roundtrip Test"
    assert book2.get_metadata("DC", "creator")[0][0] == "Author"
    assert len(book2.get_items_of_type(ITEM_DOCUMENT)) == 2
    assert len(book2.toc) == 2
