from fast_ebook import epub


def test_valid_book_no_issues():
    book = epub.EpubBook()
    book.set_identifier("valid-001")
    book.set_title("Valid Book")
    book.set_language("en")
    c1 = epub.EpubHtml(title="Ch1", file_name="ch1.xhtml")
    c1.content = "<h1>Hi</h1>"
    book.add_item(c1)
    book.add_item(epub.EpubNcx())
    book.add_item(epub.EpubNav())
    book.spine = [c1]
    assert book.validate() == []


def test_missing_identifier():
    book = epub.EpubBook()
    book.set_title("Test")
    book.set_language("en")
    c1 = epub.EpubHtml(title="Ch1", file_name="ch1.xhtml")
    c1.content = "<p>x</p>"
    book.add_item(c1)
    book.add_item(epub.EpubNav())
    book.spine = [c1]
    issues = book.validate()
    assert any("identifier" in i for i in issues)


def test_missing_title():
    book = epub.EpubBook()
    book.set_identifier("id")
    book.set_language("en")
    c1 = epub.EpubHtml(title="Ch1", file_name="ch1.xhtml")
    c1.content = "<p>x</p>"
    book.add_item(c1)
    book.add_item(epub.EpubNav())
    book.spine = [c1]
    issues = book.validate()
    assert any("title" in i for i in issues)


def test_missing_language():
    book = epub.EpubBook()
    book.set_identifier("id")
    book.set_title("Test")
    c1 = epub.EpubHtml(title="Ch1", file_name="ch1.xhtml")
    c1.content = "<p>x</p>"
    book.add_item(c1)
    book.add_item(epub.EpubNav())
    book.spine = [c1]
    issues = book.validate()
    assert any("language" in i for i in issues)


def test_empty_spine():
    book = epub.EpubBook()
    book.set_identifier("id")
    book.set_title("Test")
    book.set_language("en")
    book.add_item(epub.EpubNav())
    issues = book.validate()
    assert any("Spine" in i for i in issues)


def test_dangling_spine_ref():
    book = epub.EpubBook()
    book.set_identifier("id")
    book.set_title("Test")
    book.set_language("en")
    book.add_item(epub.EpubNav())
    book._set_spine_from_entries([("nonexistent", True)])
    issues = book.validate()
    assert any("nonexistent" in i for i in issues)


def test_no_navigation():
    book = epub.EpubBook()
    book.set_identifier("id")
    book.set_title("Test")
    book.set_language("en")
    c1 = epub.EpubHtml(title="Ch1", file_name="ch1.xhtml")
    c1.content = "<p>x</p>"
    book.add_item(c1)
    book.spine = [c1]
    issues = book.validate()
    assert any("navigation" in i.lower() for i in issues)


def test_read_book_validates(epub3_path):
    book = epub.read_epub(epub3_path)
    issues = book.validate()
    assert issues == []
