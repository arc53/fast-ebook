from fast_ebook import epub, ITEM_DOCUMENT, ITEM_IMAGE, ITEM_STYLE, ITEM_COVER, ITEM_NAVIGATION


def test_get_items_of_type_document(multi_chapter_path):
    book = epub.read_epub(multi_chapter_path)
    docs = book.get_items_of_type(ITEM_DOCUMENT)
    assert len(docs) == 3


def test_get_items_of_type_style(multi_chapter_path):
    book = epub.read_epub(multi_chapter_path)
    styles = book.get_items_of_type(ITEM_STYLE)
    assert len(styles) == 1
    assert styles[0].get_name() == "style.css"


def test_get_items_of_type_cover(multi_chapter_path):
    book = epub.read_epub(multi_chapter_path)
    covers = book.get_items_of_type(ITEM_COVER)
    assert len(covers) == 1
    assert covers[0].get_name() == "images/cover.jpg"


def test_get_items_of_type_navigation(multi_chapter_path):
    book = epub.read_epub(multi_chapter_path)
    navs = book.get_items_of_type(ITEM_NAVIGATION)
    assert len(navs) == 2  # nav.xhtml + toc.ncx


def test_get_item_with_id(multi_chapter_path):
    book = epub.read_epub(multi_chapter_path)
    item = book.get_item_with_id("ch1")
    assert item is not None
    assert item.get_id() == "ch1"


def test_get_item_with_href(multi_chapter_path):
    book = epub.read_epub(multi_chapter_path)
    item = book.get_item_with_href("chapter1.xhtml")
    assert item is not None
    assert item.get_id() == "ch1"


def test_get_item_with_id_missing(epub3_path):
    book = epub.read_epub(epub3_path)
    assert book.get_item_with_id("nonexistent") is None


def test_get_item_with_href_missing(epub3_path):
    book = epub.read_epub(epub3_path)
    assert book.get_item_with_href("nonexistent.xhtml") is None


def test_item_get_content_returns_bytes(epub3_path):
    book = epub.read_epub(epub3_path)
    item = book.get_item_with_id("ch1")
    content = item.get_content()
    assert isinstance(content, bytes)
    assert len(content) > 0


def test_item_get_content_xhtml(epub3_path):
    book = epub.read_epub(epub3_path)
    item = book.get_item_with_id("ch1")
    content = item.get_content().decode("utf-8")
    assert "<h1>" in content
    assert "Chapter 1" in content


def test_item_get_type(multi_chapter_path):
    book = epub.read_epub(multi_chapter_path)
    item = book.get_item_with_id("css")
    assert item.get_type() == ITEM_STYLE


def test_item_get_name(multi_chapter_path):
    book = epub.read_epub(multi_chapter_path)
    item = book.get_item_with_id("css")
    assert item.get_name() == "style.css"


def test_item_get_media_type(multi_chapter_path):
    book = epub.read_epub(multi_chapter_path)
    item = book.get_item_with_id("css")
    assert item.get_media_type() == "text/css"


def test_item_repr(epub3_path):
    book = epub.read_epub(epub3_path)
    item = book.get_item_with_id("ch1")
    r = repr(item)
    assert "ch1" in r
    assert "chapter1.xhtml" in r


def test_cover_image_content(multi_chapter_path):
    book = epub.read_epub(multi_chapter_path)
    covers = book.get_items_of_type(ITEM_COVER)
    content = covers[0].get_content()
    # JPEG magic bytes
    assert content[:2] == b"\xff\xd8"


def test_get_text_for_document(epub3_path):
    book = epub.read_epub(epub3_path)
    item = book.get_item_with_id("ch1")
    text = item.get_text()
    assert text is not None
    assert "Chapter 1" in text
    assert "<" not in text  # No HTML tags


def test_get_text_returns_none_for_non_document(multi_chapter_path):
    book = epub.read_epub(multi_chapter_path)
    css = book.get_item_with_id("css")
    assert css.get_text() is None
