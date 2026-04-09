from fast_ebook import epub


def test_get_title_epub2(epub2_path):
    book = epub.read_epub(epub2_path)
    titles = book.get_metadata("DC", "title")
    assert len(titles) == 1
    assert titles[0][0] == "Minimal EPUB2"


def test_get_title_epub3(epub3_path):
    book = epub.read_epub(epub3_path)
    titles = book.get_metadata("DC", "title")
    assert titles[0][0] == "Minimal EPUB3"


def test_get_language(epub2_path):
    book = epub.read_epub(epub2_path)
    langs = book.get_metadata("DC", "language")
    assert langs[0][0] == "en"


def test_get_identifier(epub2_path):
    book = epub.read_epub(epub2_path)
    ids = book.get_metadata("DC", "identifier")
    assert ids[0][0] == "test-epub2-001"


def test_get_creator(epub2_path):
    book = epub.read_epub(epub2_path)
    creators = book.get_metadata("DC", "creator")
    assert len(creators) == 1
    assert creators[0][0] == "Test Author"


def test_creator_attributes(epub2_path):
    book = epub.read_epub(epub2_path)
    creators = book.get_metadata("DC", "creator")
    attrs = creators[0][1]
    assert attrs["opf:role"] == "aut"
    assert attrs["opf:file-as"] == "Author, Test"


def test_multiple_creators(multi_chapter_path):
    book = epub.read_epub(multi_chapter_path)
    creators = book.get_metadata("DC", "creator")
    assert len(creators) == 2
    names = [c[0] for c in creators]
    assert "Author One" in names
    assert "Author Two" in names


def test_publisher(multi_chapter_path):
    book = epub.read_epub(multi_chapter_path)
    pubs = book.get_metadata("DC", "publisher")
    assert pubs[0][0] == "Test Publisher"


def test_description(multi_chapter_path):
    book = epub.read_epub(multi_chapter_path)
    desc = book.get_metadata("DC", "description")
    assert "multiple chapters" in desc[0][0]


def test_nonexistent_metadata(epub3_path):
    book = epub.read_epub(epub3_path)
    result = book.get_metadata("DC", "nonexistent")
    assert result == []


def test_opf_metadata_epub3(epub3_path):
    book = epub.read_epub(epub3_path)
    modified = book.get_metadata("OPF", "dcterms:modified")
    assert len(modified) == 1
    assert "2024" in modified[0][0]
