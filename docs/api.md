# API Reference

## Reading

### `epub.read_epub(source, options=None)`

Read an EPUB file and return an `EpubBook`.

```python
from fast_ebook import epub

# From file path
book = epub.read_epub('book.epub')

# From Path object
from pathlib import Path
book = epub.read_epub(Path('book.epub'))

# From bytes
data = open('book.epub', 'rb').read()
book = epub.read_epub(data)

# From BytesIO
import io
buf = io.BytesIO(data)
book = epub.read_epub(buf)
```

**Parameters:**
- `source` — File path (`str` or `Path`), `bytes`, `bytearray`, or file-like object with `.read()`
- `options` — Optional `dict`:
  - `ignore_ncx` (`bool`, default `False`) — Skip EPUB2 NCX table of contents parsing
  - `ignore_nav` (`bool`, default `False`) — Skip EPUB3 Nav document parsing
  - `lazy` (`bool`, default `False`) — Defer item content loading until first access. Significantly faster for metadata-only workflows

**Returns:** `EpubBook`

### `epub.open(source, options=None)`

Same as `read_epub`, but intended for use as a context manager.

```python
with epub.open('book.epub') as book:
    print(book.get_metadata('DC', 'title'))
```

### `epub.read_epubs(paths, workers=None, options=None)`

Read multiple EPUB files in parallel using Rayon thread pool. GIL is released during processing.

```python
from pathlib import Path
from fast_ebook import epub

paths = list(Path('library/').glob('*.epub'))
books = epub.read_epubs(paths, workers=4)
```

**Parameters:**
- `paths` — List of file paths (`str` or `Path`)
- `workers` — Number of parallel threads (`None` = auto, uses all cores)
- `options` — Same as `read_epub`

**Returns:** `list[EpubBook]`. Raises on first error.

---

## Writing

### `epub.write_epub(target, book)`

Write an `EpubBook` to a file or BytesIO.

```python
# To file
epub.write_epub('output.epub', book)

# To BytesIO
import io
buf = io.BytesIO()
epub.write_epub(buf, book)
epub_bytes = buf.getvalue()
```

**Parameters:**
- `target` — File path (`str` or `Path`) or writable file-like object
- `book` — `EpubBook` instance

---

## EpubBook

### Constructor

```python
book = epub.EpubBook()
```

Creates an empty book for building from scratch.

### Metadata Methods

```python
book.set_identifier('isbn-123')
book.set_title('My Book')
book.set_language('en')
book.add_author('Jane Doe')
book.add_author('Illustrator', file_as='Doe, Ill', role='ill', uid='illust')
book.add_metadata('DC', 'description', 'A great book')
book.add_metadata('DC', 'publisher', 'Publisher Inc')
```

### Reading Metadata

```python
# Returns list of (value, attributes_dict) tuples
titles = book.get_metadata('DC', 'title')
# [('My Book', {})]

creators = book.get_metadata('DC', 'creator')
# [('Jane Doe', {}), ('Illustrator', {'opf:role': 'ill', 'opf:file-as': 'Doe, Ill'})]
```

### Item Access

```python
# All items
items = book.get_items()

# Filter by type
from fast_ebook import ITEM_DOCUMENT, ITEM_IMAGE, ITEM_STYLE
docs = book.get_items_of_type(ITEM_DOCUMENT)
images = book.get_items_of_type(ITEM_IMAGE)

# Lookup by ID or href
item = book.get_item_with_id('chapter1')
item = book.get_item_with_href('text/chapter1.xhtml')
```

### Adding Items

```python
# XHTML chapter
c1 = epub.EpubHtml(title='Chapter 1', file_name='ch1.xhtml')
c1.content = '<h1>Hello</h1><p>World</p>'  # fragments auto-wrapped in XHTML

# Image
img = epub.EpubImage(uid='cover', file_name='images/cover.jpg',
                     media_type='image/jpeg', content=open('cover.jpg', 'rb').read())

# CSS
css = epub.EpubCss(uid='style', file_name='style.css', content='body { margin: 1em; }')

# Navigation (auto-generated at write time)
book.add_item(c1)
book.add_item(img)
book.add_item(css)
book.add_item(epub.EpubNcx())   # EPUB2 NCX
book.add_item(epub.EpubNav())   # EPUB3 Nav document
```

### Cover Image

```python
book.set_cover('cover.jpg', open('cover.jpg', 'rb').read())
```

### Table of Contents

```python
book.toc = [
    epub.Link('ch1.xhtml', 'Chapter 1', 'ch1'),
    epub.Link('ch2.xhtml', 'Chapter 2', 'ch2'),
    # Nested sections
    (epub.Section('Part 2'), [
        epub.Link('ch3.xhtml', 'Chapter 3', 'ch3'),
        epub.Link('ch4.xhtml', 'Chapter 4', 'ch4'),
    ]),
]
```

Reading the ToC returns `TocEntry` objects:

```python
for entry in book.toc:
    print(entry.title, entry.href)
    for child in entry.children:
        print(f'  {child.title} -> {child.href}')
```

### Spine (Reading Order)

```python
book.spine = ['nav', c1, c2, c3]  # mix of string IDs and item objects

# Reading spine
for idref, linear in book.get_spine():
    print(idref, 'linear' if linear else 'non-linear')
```

### Validation

```python
issues = book.validate()
if issues:
    for issue in issues:
        print(f'  - {issue}')
else:
    print('Valid EPUB')
```

**Checks performed:**
- Required DC metadata (identifier, title, language)
- Spine is non-empty
- Spine references point to existing manifest items
- No duplicate item IDs or hrefs
- At least one navigation item present

### Markdown Conversion

```python
md = book.to_markdown()
# Returns the entire book as a single Markdown string
# Follows spine order, converts headings/bold/italic/links/lists
```

### Context Manager

```python
with epub.open('book.epub') as book:
    print(book.get_metadata('DC', 'title'))
# No cleanup needed — Rust handles memory
```

---

## EpubItem

Returned by `book.get_items()`, `get_items_of_type()`, `get_item_with_id()`, `get_item_with_href()`.

```python
item.get_id()           # Manifest ID (str)
item.get_name()         # Href within EPUB (str)
item.get_type()         # ITEM_* constant (int)
item.get_media_type()   # MIME type (str)
item.get_content()      # Raw bytes (bytes)
item.get_text()         # Text content, tags stripped (str | None, only for documents)
```

---

## Item Type Constants

```python
import fast_ebook

fast_ebook.ITEM_UNKNOWN     # 0
fast_ebook.ITEM_IMAGE       # 1
fast_ebook.ITEM_STYLE       # 2
fast_ebook.ITEM_SCRIPT      # 3
fast_ebook.ITEM_NAVIGATION  # 4
fast_ebook.ITEM_VECTOR      # 5
fast_ebook.ITEM_FONT        # 6
fast_ebook.ITEM_VIDEO       # 7
fast_ebook.ITEM_AUDIO       # 8
fast_ebook.ITEM_DOCUMENT    # 9
fast_ebook.ITEM_COVER       # 10
fast_ebook.ITEM_SMIL        # 11
```

---

## Convenience Classes (for building EPUBs)

### `epub.EpubHtml(uid=None, title='', file_name='', lang=None, content=b'', media_type='application/xhtml+xml')`

XHTML content document. Setting `content` to an HTML fragment (e.g., `'<h1>Hi</h1>'`) auto-wraps it in a valid XHTML skeleton at write time.

### `epub.EpubImage(uid=None, file_name='', media_type='', content=b'')`

Image item (JPEG, PNG, GIF, WebP, etc.).

### `epub.EpubCss(uid=None, file_name='', content=b'')`

CSS stylesheet. `media_type` defaults to `text/css`.

### `epub.EpubNcx()`

Sentinel — NCX content is auto-generated from `book.toc` at write time.

### `epub.EpubNav()`

Sentinel — Nav XHTML content is auto-generated from `book.toc` at write time.

### `epub.Link(href, title, uid='')`

Table of contents entry linking to a document.

### `epub.Section(title)`

Table of contents section heading, used with tuples: `(Section('Part 1'), [Link(...), ...])`

---

## Compatibility Layer

For migrating from ebooklib with minimal code changes:

```python
# One-line change
import fast_ebook.compat as ebooklib
from fast_ebook.compat import epub

# All ebooklib API names available
book = epub.read_epub('book.epub')
for img in book.get_items_of_type(ebooklib.ITEM_IMAGE):
    ...
```

---

## CLI

```bash
# Metadata
fast-ebook info book.epub
fast-ebook info book.epub --format json

# Validation
fast-ebook validate book.epub
fast-ebook validate *.epub --format json

# Extract items
fast-ebook extract book.epub --output-dir ./out
fast-ebook extract book.epub --output-dir ./imgs --type images

# Convert to Markdown
fast-ebook convert book.epub -o book.md
fast-ebook convert book.epub > book.md

# Batch scan (parallel)
fast-ebook scan library/ --workers 8
fast-ebook scan library/ --format csv > catalog.csv
fast-ebook scan library/ --format json
```
