import io
from pathlib import Path

from fast_ebook import epub, ITEM_DOCUMENT

FIXTURES = Path(__file__).parent / "fixtures"


def test_read_from_bytes():
    data = (FIXTURES / "minimal_epub3.epub").read_bytes()
    book = epub.read_epub(data)
    assert book.get_metadata("DC", "title")[0][0] == "Minimal EPUB3"


def test_read_from_bytesio():
    data = (FIXTURES / "minimal_epub3.epub").read_bytes()
    buf = io.BytesIO(data)
    book = epub.read_epub(buf)
    assert book.get_metadata("DC", "title")[0][0] == "Minimal EPUB3"


def test_write_to_bytesio():
    book = epub.read_epub(str(FIXTURES / "minimal_epub3.epub"))
    buf = io.BytesIO()
    epub.write_epub(buf, book)
    assert buf.tell() > 0
    buf.seek(0)
    # Verify it's a valid ZIP/EPUB
    assert buf.read(2) == b"PK"


def test_roundtrip_through_memory():
    # Read from file
    book1 = epub.read_epub(str(FIXTURES / "multi_chapter.epub"))

    # Write to bytes
    buf = io.BytesIO()
    epub.write_epub(buf, book1)

    # Read from bytes
    book2 = epub.read_epub(buf.getvalue())

    assert book2.get_metadata("DC", "title") == book1.get_metadata("DC", "title")
    assert len(book2.get_items()) >= 3


def test_read_bytes_with_options():
    data = (FIXTURES / "minimal_epub2.epub").read_bytes()
    book = epub.read_epub(data, options={"ignore_ncx": True})
    assert book.get_metadata("DC", "title")[0][0] == "Minimal EPUB2"
    assert len(book.toc) == 0  # NCX ignored, no nav document in EPUB2


def test_write_to_bytesio_and_read_back():
    # Create a book from scratch
    book = epub.EpubBook()
    book.set_identifier("bytesio-test")
    book.set_title("BytesIO Test")
    book.set_language("en")
    c1 = epub.EpubHtml(title="Ch1", file_name="ch1.xhtml")
    c1.content = "<h1>Test</h1>"
    book.add_item(c1)
    book.add_item(epub.EpubNcx())
    book.add_item(epub.EpubNav())
    book.toc = [epub.Link("ch1.xhtml", "Ch1", "ch1")]
    book.spine = [c1]

    buf = io.BytesIO()
    epub.write_epub(buf, book)

    book2 = epub.read_epub(buf.getvalue())
    assert book2.get_metadata("DC", "title")[0][0] == "BytesIO Test"
