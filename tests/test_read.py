import pytest
from fast_ebook import epub


def test_read_epub2(epub2_path):
    book = epub.read_epub(epub2_path)
    assert book is not None
    assert len(book.get_items()) > 0


def test_read_epub3(epub3_path):
    book = epub.read_epub(epub3_path)
    assert book is not None
    assert len(book.get_items()) > 0


def test_read_multi_chapter(multi_chapter_path):
    book = epub.read_epub(multi_chapter_path)
    assert len(book.get_items()) == 7  # 3 chapters + nav + ncx + css + cover


def test_read_nonexistent_raises():
    with pytest.raises(ValueError):
        epub.read_epub("/nonexistent/path.epub")


def test_read_invalid_file(tmp_path):
    bad_file = tmp_path / "not_an_epub.txt"
    bad_file.write_text("this is not an epub")
    with pytest.raises(ValueError):
        epub.read_epub(str(bad_file))


def test_spine_not_empty(epub3_path):
    book = epub.read_epub(epub3_path)
    spine = book.get_spine()
    assert len(spine) > 0
    assert isinstance(spine[0], tuple)
    assert isinstance(spine[0][0], str)  # idref
    assert isinstance(spine[0][1], bool)  # linear


def test_repr(epub3_path):
    book = epub.read_epub(epub3_path)
    assert "Minimal EPUB3" in repr(book)


def test_read_path_object(epub3_path):
    from pathlib import Path
    book = epub.read_epub(Path(epub3_path))
    assert book is not None


def test_read_with_ignore_ncx(epub2_path):
    book = epub.read_epub(epub2_path, options={"ignore_ncx": True})
    assert book is not None
    # EPUB2 only has NCX, so toc should be empty when ignored
    assert len(book.toc) == 0


def test_read_with_ignore_nav(epub3_path):
    book = epub.read_epub(epub3_path, options={"ignore_nav": True})
    assert book is not None
    # EPUB3 with no NCX fallback → empty toc
    assert len(book.toc) == 0


def test_context_manager(epub3_path):
    with epub.open(epub3_path) as book:
        assert book.get_metadata("DC", "title")[0][0] == "Minimal EPUB3"
        assert len(book.get_items()) > 0


def test_context_manager_with_options(epub2_path):
    with epub.open(epub2_path, options={"ignore_ncx": True}) as book:
        assert len(book.toc) == 0
