import zipfile

import pytest
from fast_ebook import epub, ITEM_DOCUMENT, ITEM_IMAGE, ITEM_COVER, ITEM_STYLE, ITEM_NAVIGATION


def _make_minimal_book():
    """Helper: create a minimal valid book."""
    book = epub.EpubBook()
    book.set_identifier("test-123")
    book.set_title("Test Book")
    book.set_language("en")

    c1 = epub.EpubHtml(title="Chapter 1", file_name="chap_01.xhtml")
    c1.content = "<h1>Hello</h1><p>World</p>"
    book.add_item(c1)
    book.add_item(epub.EpubNcx())
    book.add_item(epub.EpubNav())

    book.toc = [epub.Link("chap_01.xhtml", "Chapter 1", "ch1")]
    book.spine = ["nav", c1]
    return book


class TestWriteMinimal:
    def test_write_and_read_back(self, tmp_path):
        book = _make_minimal_book()
        out = tmp_path / "test.epub"
        epub.write_epub(str(out), book)

        book2 = epub.read_epub(str(out))
        assert book2.get_metadata("DC", "title")[0][0] == "Test Book"
        assert book2.get_metadata("DC", "identifier")[0][0] == "test-123"
        assert book2.get_metadata("DC", "language")[0][0] == "en"

    def test_items_present(self, tmp_path):
        book = _make_minimal_book()
        out = tmp_path / "test.epub"
        epub.write_epub(str(out), book)

        book2 = epub.read_epub(str(out))
        assert len(book2.get_items()) >= 3  # chapter + ncx + nav

    def test_spine_preserved(self, tmp_path):
        book = _make_minimal_book()
        out = tmp_path / "test.epub"
        epub.write_epub(str(out), book)

        book2 = epub.read_epub(str(out))
        spine = book2.get_spine()
        assert len(spine) == 2
        idrefs = [s[0] for s in spine]
        assert "nav" in idrefs
        assert "chap_01_xhtml" in idrefs

    def test_toc_preserved(self, tmp_path):
        book = _make_minimal_book()
        out = tmp_path / "test.epub"
        epub.write_epub(str(out), book)

        book2 = epub.read_epub(str(out))
        toc = book2.toc
        assert len(toc) == 1
        assert toc[0].title == "Chapter 1"
        assert toc[0].href == "chap_01.xhtml"


class TestZipStructure:
    def test_mimetype_first(self, tmp_path):
        book = _make_minimal_book()
        out = tmp_path / "test.epub"
        epub.write_epub(str(out), book)

        with zipfile.ZipFile(str(out)) as zf:
            assert zf.namelist()[0] == "mimetype"

    def test_mimetype_uncompressed(self, tmp_path):
        book = _make_minimal_book()
        out = tmp_path / "test.epub"
        epub.write_epub(str(out), book)

        with zipfile.ZipFile(str(out)) as zf:
            info = zf.getinfo("mimetype")
            assert info.compress_type == zipfile.ZIP_STORED

    def test_mimetype_content(self, tmp_path):
        book = _make_minimal_book()
        out = tmp_path / "test.epub"
        epub.write_epub(str(out), book)

        with zipfile.ZipFile(str(out)) as zf:
            assert zf.read("mimetype") == b"application/epub+zip"

    def test_container_xml_present(self, tmp_path):
        book = _make_minimal_book()
        out = tmp_path / "test.epub"
        epub.write_epub(str(out), book)

        with zipfile.ZipFile(str(out)) as zf:
            assert "META-INF/container.xml" in zf.namelist()
            content = zf.read("META-INF/container.xml").decode()
            assert "EPUB/content.opf" in content

    def test_opf_present(self, tmp_path):
        book = _make_minimal_book()
        out = tmp_path / "test.epub"
        epub.write_epub(str(out), book)

        with zipfile.ZipFile(str(out)) as zf:
            assert "EPUB/content.opf" in zf.namelist()


class TestValidation:
    def test_missing_identifier_raises(self):
        import io
        book = epub.EpubBook()
        book.set_title("Test")
        book.set_language("en")
        c1 = epub.EpubHtml(title="Ch1", file_name="ch1.xhtml")
        c1.content = "<p>x</p>"
        book.add_item(c1)
        book.spine = [c1]
        with pytest.raises(ValueError, match="identifier"):
            epub.write_epub(io.BytesIO(), book)

    def test_missing_title_raises(self):
        import io
        book = epub.EpubBook()
        book.set_identifier("id")
        book.set_language("en")
        c1 = epub.EpubHtml(title="Ch1", file_name="ch1.xhtml")
        c1.content = "<p>x</p>"
        book.add_item(c1)
        book.spine = [c1]
        with pytest.raises(ValueError, match="title"):
            epub.write_epub(io.BytesIO(), book)

    def test_missing_language_raises(self):
        import io
        book = epub.EpubBook()
        book.set_identifier("id")
        book.set_title("Test")
        c1 = epub.EpubHtml(title="Ch1", file_name="ch1.xhtml")
        c1.content = "<p>x</p>"
        book.add_item(c1)
        book.spine = [c1]
        with pytest.raises(ValueError, match="language"):
            epub.write_epub(io.BytesIO(), book)

    def test_empty_spine_raises(self):
        import io
        book = epub.EpubBook()
        book.set_identifier("id")
        book.set_title("Test")
        book.set_language("en")
        with pytest.raises(ValueError, match="spine"):
            epub.write_epub(io.BytesIO(), book)


class TestAuthors:
    def test_single_author(self, tmp_path):
        book = _make_minimal_book()
        book.add_author("Jane Doe")
        out = tmp_path / "test.epub"
        epub.write_epub(str(out), book)

        book2 = epub.read_epub(str(out))
        creators = book2.get_metadata("DC", "creator")
        assert any(c[0] == "Jane Doe" for c in creators)

    def test_author_with_attributes(self, tmp_path):
        book = _make_minimal_book()
        book.add_author("Illustrator", file_as="Doe, Ill", role="ill", uid="coauthor")
        out = tmp_path / "test.epub"
        epub.write_epub(str(out), book)

        book2 = epub.read_epub(str(out))
        creators = book2.get_metadata("DC", "creator")
        ill = [c for c in creators if c[0] == "Illustrator"]
        assert len(ill) == 1
        assert ill[0][1].get("opf:role") == "ill"
        assert ill[0][1].get("opf:file-as") == "Doe, Ill"

    def test_multiple_authors(self, tmp_path):
        book = _make_minimal_book()
        book.add_author("Author One")
        book.add_author("Author Two")
        out = tmp_path / "test.epub"
        epub.write_epub(str(out), book)

        book2 = epub.read_epub(str(out))
        creators = book2.get_metadata("DC", "creator")
        names = [c[0] for c in creators]
        assert "Author One" in names
        assert "Author Two" in names


class TestCover:
    def test_set_cover(self, tmp_path):
        book = _make_minimal_book()
        # Tiny valid JPEG header
        fake_jpg = b"\xff\xd8\xff\xe0" + b"\x00" * 100
        book.set_cover("images/cover.jpg", fake_jpg)
        out = tmp_path / "test.epub"
        epub.write_epub(str(out), book)

        book2 = epub.read_epub(str(out))
        covers = book2.get_items_of_type(ITEM_COVER)
        assert len(covers) == 1
        assert covers[0].get_name() == "images/cover.jpg"
        assert covers[0].get_content()[:4] == b"\xff\xd8\xff\xe0"


class TestMultipleChapters:
    def test_three_chapters(self, tmp_path):
        book = epub.EpubBook()
        book.set_identifier("multi-001")
        book.set_title("Multi Chapter")
        book.set_language("en")

        chapters = []
        for i in range(1, 4):
            c = epub.EpubHtml(title=f"Chapter {i}", file_name=f"ch{i}.xhtml")
            c.content = f"<h1>Chapter {i}</h1><p>Content {i}</p>"
            book.add_item(c)
            chapters.append(c)

        css = epub.EpubCss(uid="style", file_name="style.css", content="body { margin: 1em; }")
        book.add_item(css)
        book.add_item(epub.EpubNcx())
        book.add_item(epub.EpubNav())

        book.toc = [epub.Link(f"ch{i}.xhtml", f"Chapter {i}", f"ch{i}") for i in range(1, 4)]
        book.spine = ["nav"] + chapters
        out = tmp_path / "test.epub"
        epub.write_epub(str(out), book)

        book2 = epub.read_epub(str(out))
        docs = book2.get_items_of_type(ITEM_DOCUMENT)
        assert len(docs) == 3
        styles = book2.get_items_of_type(ITEM_STYLE)
        assert len(styles) == 1
        assert len(book2.toc) == 3


class TestNestedToc:
    def test_section_tuple_toc(self, tmp_path):
        book = epub.EpubBook()
        book.set_identifier("nested-001")
        book.set_title("Nested ToC")
        book.set_language("en")

        c1 = epub.EpubHtml(title="Ch1", file_name="ch1.xhtml")
        c1.content = "<h1>Ch1</h1>"
        c2 = epub.EpubHtml(title="Ch2", file_name="ch2.xhtml")
        c2.content = "<h1>Ch2</h1>"
        book.add_item(c1)
        book.add_item(c2)
        book.add_item(epub.EpubNcx())
        book.add_item(epub.EpubNav())

        book.toc = [
            (epub.Section("Part 1"), [
                epub.Link("ch1.xhtml", "Chapter 1", "ch1"),
                epub.Link("ch2.xhtml", "Chapter 2", "ch2"),
            ]),
        ]
        book.spine = [c1, c2]
        out = tmp_path / "test.epub"
        epub.write_epub(str(out), book)

        book2 = epub.read_epub(str(out))
        toc = book2.toc
        assert len(toc) == 1
        assert toc[0].title == "Part 1"
        assert len(toc[0].children) == 2
        assert toc[0].children[0].title == "Chapter 1"
        assert toc[0].children[1].title == "Chapter 2"


class TestHtmlWrapping:
    def test_fragment_gets_wrapped(self, tmp_path):
        book = _make_minimal_book()
        out = tmp_path / "test.epub"
        epub.write_epub(str(out), book)

        book2 = epub.read_epub(str(out))
        doc = book2.get_items_of_type(ITEM_DOCUMENT)[0]
        content = doc.get_content().decode()
        assert "<?xml" in content
        assert "<html" in content
        assert "<body>" in content
        assert "<h1>Hello</h1>" in content

    def test_full_xhtml_not_double_wrapped(self, tmp_path):
        book = epub.EpubBook()
        book.set_identifier("full-001")
        book.set_title("Full Doc")
        book.set_language("en")

        full_xhtml = '<?xml version="1.0"?><html xmlns="http://www.w3.org/1999/xhtml"><head/><body><p>Hi</p></body></html>'
        c1 = epub.EpubHtml(title="Ch1", file_name="ch1.xhtml")
        c1.content = full_xhtml
        book.add_item(c1)
        book.add_item(epub.EpubNcx())
        book.add_item(epub.EpubNav())
        book.toc = [epub.Link("ch1.xhtml", "Ch1", "ch1")]
        book.spine = [c1]
        out = tmp_path / "test.epub"
        epub.write_epub(str(out), book)

        book2 = epub.read_epub(str(out))
        doc = book2.get_items_of_type(ITEM_DOCUMENT)[0]
        content = doc.get_content().decode()
        # Should not be double-wrapped
        assert content.count("<html") == 1
        assert content.count("<body>") == 1


class TestRoundtrip:
    def test_roundtrip_epub2(self, epub2_path, tmp_path):
        """Read EPUB2 fixture, write it, read again, compare."""
        book1 = epub.read_epub(epub2_path)
        out = tmp_path / "roundtrip.epub"
        epub.write_epub(str(out), book1)

        book2 = epub.read_epub(str(out))
        assert book2.get_metadata("DC", "title") == book1.get_metadata("DC", "title")
        assert book2.get_metadata("DC", "language") == book1.get_metadata("DC", "language")
        assert len(book2.toc) == len(book1.toc)

    def test_roundtrip_epub3(self, epub3_path, tmp_path):
        """Read EPUB3 fixture, write it, read again, compare."""
        book1 = epub.read_epub(epub3_path)
        out = tmp_path / "roundtrip.epub"
        epub.write_epub(str(out), book1)

        book2 = epub.read_epub(str(out))
        assert book2.get_metadata("DC", "title") == book1.get_metadata("DC", "title")
        assert len(book2.get_items()) >= 2  # at least chapter + nav


class TestEbooklibExample:
    def test_plan_md_write_example(self, tmp_path):
        """The exact write example from PLAN.md works end-to-end."""
        book = epub.EpubBook()
        book.set_identifier("id123")
        book.set_title("My Book")
        book.set_language("en")
        book.add_author("Author Name")
        book.add_author("Illustrator", file_as="Illustrator Name", role="ill", uid="coauthor")
        book.add_metadata("DC", "description", "A description")

        c1 = epub.EpubHtml(title="Intro", file_name="chap_01.xhtml", lang="en")
        c1.content = "<h1>Hello</h1><p>World</p>"

        book.add_item(c1)

        book.toc = [
            epub.Link("chap_01.xhtml", "Introduction", "intro"),
            (epub.Section("Part 1"), [
                epub.Link("chap_01.xhtml", "Intro Again", "intro2"),
            ]),
        ]
        book.spine = ["nav", c1]

        book.add_item(epub.EpubNcx())
        book.add_item(epub.EpubNav())

        out = tmp_path / "output.epub"
        epub.write_epub(str(out), book)

        # Read back and verify
        book2 = epub.read_epub(str(out))
        assert book2.get_metadata("DC", "title")[0][0] == "My Book"
        assert len(book2.get_metadata("DC", "creator")) == 2
        assert book2.get_metadata("DC", "description")[0][0] == "A description"
        assert len(book2.toc) == 2
        assert book2.toc[1].title == "Part 1"
        assert len(book2.toc[1].children) == 1


class TestCssItem:
    def test_css_roundtrip(self, tmp_path):
        book = _make_minimal_book()
        css = epub.EpubCss(uid="style", file_name="style.css",
                           content="body { font-family: serif; }")
        book.add_item(css)
        out = tmp_path / "test.epub"
        epub.write_epub(str(out), book)

        book2 = epub.read_epub(str(out))
        styles = book2.get_items_of_type(ITEM_STYLE)
        assert len(styles) == 1
        assert b"font-family" in styles[0].get_content()


class TestImageItem:
    def test_image_roundtrip(self, tmp_path):
        book = _make_minimal_book()
        fake_png = b"\x89PNG\r\n\x1a\n" + b"\x00" * 50
        img = epub.EpubImage(uid="img1", file_name="images/photo.png",
                             media_type="image/png", content=fake_png)
        book.add_item(img)
        out = tmp_path / "test.epub"
        epub.write_epub(str(out), book)

        book2 = epub.read_epub(str(out))
        images = book2.get_items_of_type(ITEM_IMAGE)
        assert len(images) == 1
        assert images[0].get_content()[:4] == b"\x89PNG"
