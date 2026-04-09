"""Tests for parallel batch reading with Rayon."""

from pathlib import Path

import pytest
from fast_ebook import epub

FIXTURES = Path(__file__).parent / "fixtures"


def test_read_epubs_basic():
    paths = [
        str(FIXTURES / "minimal_epub2.epub"),
        str(FIXTURES / "minimal_epub3.epub"),
        str(FIXTURES / "multi_chapter.epub"),
        str(FIXTURES / "nested_toc.epub"),
    ]
    books = epub.read_epubs(paths)
    assert len(books) == 4
    titles = [b.get_metadata("DC", "title")[0][0] for b in books]
    assert "Minimal EPUB2" in titles
    assert "Minimal EPUB3" in titles


def test_read_epubs_with_workers():
    paths = [
        str(FIXTURES / "minimal_epub2.epub"),
        str(FIXTURES / "minimal_epub3.epub"),
    ]
    books = epub.read_epubs(paths, workers=2)
    assert len(books) == 2


def test_read_epubs_single_worker():
    paths = [str(FIXTURES / "minimal_epub3.epub")]
    books = epub.read_epubs(paths, workers=1)
    assert len(books) == 1
    assert books[0].get_metadata("DC", "title")[0][0] == "Minimal EPUB3"


def test_read_epubs_matches_sequential():
    paths = [
        str(FIXTURES / "minimal_epub2.epub"),
        str(FIXTURES / "multi_chapter.epub"),
    ]
    # Sequential
    seq = [epub.read_epub(p) for p in paths]
    # Parallel
    par = epub.read_epubs(paths, workers=2)

    for s, p in zip(seq, par):
        assert s.get_metadata("DC", "title") == p.get_metadata("DC", "title")
        assert len(s.get_items()) == len(p.get_items())


def test_read_epubs_bad_path_raises():
    paths = [
        str(FIXTURES / "minimal_epub3.epub"),
        "/nonexistent/path.epub",
    ]
    with pytest.raises(ValueError):
        epub.read_epubs(paths)


def test_read_epubs_with_options():
    paths = [str(FIXTURES / "minimal_epub2.epub")]
    books = epub.read_epubs(paths, options={"ignore_ncx": True})
    assert len(books[0].toc) == 0


def test_read_epubs_empty_list():
    books = epub.read_epubs([])
    assert books == []
