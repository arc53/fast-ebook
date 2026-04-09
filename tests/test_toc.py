from fast_ebook import epub


def test_epub2_toc_from_ncx(epub2_path):
    book = epub.read_epub(epub2_path)
    toc = book.toc
    assert len(toc) == 1
    assert toc[0].title == "Chapter 1"
    assert toc[0].href == "chapter1.xhtml"


def test_epub3_toc_from_nav(epub3_path):
    book = epub.read_epub(epub3_path)
    toc = book.toc
    assert len(toc) == 1
    assert toc[0].title == "Chapter 1"


def test_multi_chapter_toc(multi_chapter_path):
    book = epub.read_epub(multi_chapter_path)
    toc = book.toc
    assert len(toc) == 3
    titles = [e.title for e in toc]
    assert "Introduction" in titles
    assert "The Middle" in titles
    assert "Conclusion" in titles


def test_toc_hrefs(multi_chapter_path):
    book = epub.read_epub(multi_chapter_path)
    toc = book.toc
    hrefs = [e.href for e in toc]
    assert "chapter1.xhtml" in hrefs
    assert "chapter2.xhtml" in hrefs
    assert "chapter3.xhtml" in hrefs


def test_nested_toc(nested_toc_path):
    book = epub.read_epub(nested_toc_path)
    toc = book.toc
    assert len(toc) == 2  # Part 1 and Part 2

    part1 = toc[0]
    assert part1.title == "Part 1: Beginning"
    assert len(part1.children) == 2
    assert part1.children[0].title == "Chapter 1"
    assert part1.children[1].title == "Chapter 2"

    part2 = toc[1]
    assert part2.title == "Part 2: End"
    assert len(part2.children) == 1
    assert part2.children[0].title == "Chapter 3"


def test_toc_entry_repr(epub3_path):
    book = epub.read_epub(epub3_path)
    entry = book.toc[0]
    r = repr(entry)
    assert "Chapter 1" in r


def test_empty_children(epub3_path):
    book = epub.read_epub(epub3_path)
    entry = book.toc[0]
    assert entry.children == []
