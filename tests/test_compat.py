"""Tests for the ebooklib compatibility layer."""


def test_compat_import():
    from fast_ebook.compat import epub
    import fast_ebook.compat as ebooklib

    assert hasattr(ebooklib, "ITEM_IMAGE")
    assert hasattr(ebooklib, "ITEM_DOCUMENT")
    assert hasattr(epub, "read_epub")
    assert hasattr(epub, "write_epub")
    assert hasattr(epub, "EpubBook")
    assert hasattr(epub, "EpubHtml")
    assert hasattr(epub, "EpubImage")
    assert hasattr(epub, "EpubNcx")
    assert hasattr(epub, "EpubNav")
    assert hasattr(epub, "Link")
    assert hasattr(epub, "Section")


def test_compat_constants():
    import fast_ebook.compat as ebooklib
    assert ebooklib.ITEM_IMAGE == 1
    assert ebooklib.ITEM_DOCUMENT == 9
    assert ebooklib.ITEM_COVER == 10


def test_compat_read(epub3_path):
    from fast_ebook.compat import epub
    book = epub.read_epub(epub3_path)
    assert book.get_metadata("DC", "title")[0][0] == "Minimal EPUB3"


def test_compat_read_write_flow(tmp_path):
    """Run the exact ebooklib README example through compat layer."""
    from fast_ebook.compat import epub

    book = epub.EpubBook()
    book.set_identifier("id123456")
    book.set_title("Sample book")
    book.set_language("en")
    book.add_author("Author Authorowski")

    c1 = epub.EpubHtml(title="Intro", file_name="chap_01.xhtml", lang="en")
    c1.content = "<h1>Intro heading</h1><p>Hello world.</p>"
    book.add_item(c1)

    book.toc = [epub.Link("chap_01.xhtml", "Introduction", "intro")]
    book.spine = ["nav", c1]
    book.add_item(epub.EpubNcx())
    book.add_item(epub.EpubNav())

    out = tmp_path / "compat_test.epub"
    epub.write_epub(str(out), book)

    book2 = epub.read_epub(str(out))
    assert book2.get_metadata("DC", "title")[0][0] == "Sample book"
