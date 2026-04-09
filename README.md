# fast-ebook

Rust-powered EPUB2/EPUB3 library for Python. Fast reading, writing, validation, and markdown conversion and a neat MIT license.

## Installation

```bash
pip install fast-ebook
```

## Quick Start

### Reading an EPUB

```python
from fast_ebook import epub
import fast_ebook

book = epub.read_epub('book.epub')

# Metadata
print(book.get_metadata('DC', 'title'))
print(book.get_metadata('DC', 'creator'))

# Iterate items
for item in book.get_items():
    print(item.get_id(), item.get_name(), item.get_type())

# Filter by type
for img in book.get_items_of_type(fast_ebook.ITEM_IMAGE):
    print(img.get_name(), len(img.get_content()), 'bytes')

# Lookup by ID or href
item = book.get_item_with_id('chapter1')
item = book.get_item_with_href('text/chapter1.xhtml')

# Table of contents
for entry in book.toc:
    print(entry.title, entry.href)
```

### Writing an EPUB

```python
from fast_ebook import epub

book = epub.EpubBook()
book.set_identifier('id123')
book.set_title('My Book')
book.set_language('en')
book.add_author('Author Name')

c1 = epub.EpubHtml(title='Intro', file_name='chap_01.xhtml', lang='en')
c1.content = '<h1>Hello</h1><p>World</p>'

book.add_item(c1)
book.add_item(epub.EpubNcx())
book.add_item(epub.EpubNav())

book.toc = [epub.Link('chap_01.xhtml', 'Introduction', 'intro')]
book.spine = ['nav', c1]

epub.write_epub('output.epub', book)
```

### Reading from / writing to BytesIO

```python
import io
from fast_ebook import epub

# Read from bytes
with open('book.epub', 'rb') as f:
    book = epub.read_epub(f)

# Write to BytesIO
buf = io.BytesIO()
epub.write_epub(buf, book)
epub_bytes = buf.getvalue()

# Read from raw bytes
book = epub.read_epub(epub_bytes)
```

### Context Manager

```python
from fast_ebook import epub

with epub.open('book.epub') as book:
    print(book.get_metadata('DC', 'title'))
```

### Parallel Batch Processing

Rust + Rayon gives true parallel EPUB processing with the GIL released.

```python
from pathlib import Path
from fast_ebook import epub

paths = list(Path('library/').glob('*.epub'))
books = epub.read_epubs(paths, workers=4)

for book in books:
    title = book.get_metadata('DC', 'title')[0][0]
    print(title)
```

### Validation

```python
from fast_ebook import epub

book = epub.read_epub('book.epub')
issues = book.validate()
if issues:
    for issue in issues:
        print(f"  - {issue}")
else:
    print("Valid EPUB")
```

### EPUB to Markdown

```python
from fast_ebook import epub

book = epub.read_epub('book.epub')
md = book.to_markdown()

# Write to file
with open('book.md', 'w') as f:
    f.write(md)
```

Converts the entire book to Markdown following spine order. Handles headings, bold, italic, links, lists, and HTML entities. Runs in Rust — converts War and Peace (368 chapters) in 71ms.

### Read Options

```python
from fast_ebook import epub

# Skip NCX parsing (EPUB2 table of contents)
book = epub.read_epub('book.epub', options={'ignore_ncx': True})

# Skip Nav document parsing (EPUB3 table of contents)
book = epub.read_epub('book.epub', options={'ignore_nav': True})
```

## Migration from ebooklib

fast-ebook's API mirrors ebooklib's public interface. For most code, you only need to change the import:

```python
# Before (ebooklib)
from ebooklib import epub
import ebooklib
book = epub.read_epub('book.epub')
for img in book.get_items_of_type(ebooklib.ITEM_IMAGE):
    ...

# After (fast-ebook)
from fast_ebook import epub
import fast_ebook
book = epub.read_epub('book.epub')
for img in book.get_items_of_type(fast_ebook.ITEM_IMAGE):
    ...
```

Or use the compatibility layer for a one-line change:

```python
# Minimal change
import fast_ebook.compat as ebooklib
from fast_ebook.compat import epub
# ... rest of your code works unchanged
```

## CLI Tool

A standalone binary (no Python required) is also available:

```bash
# Print metadata
fast-ebook info book.epub
fast-ebook info book.epub --format json

# Validate against EPUB spec
fast-ebook validate book.epub
fast-ebook validate *.epub --format json

# Convert to Markdown
fast-ebook convert book.epub -o book.md
fast-ebook convert book.epub > book.md

# Extract items
fast-ebook extract book.epub --output-dir ./out
fast-ebook extract book.epub --output-dir ./imgs --type images

# Batch scan (parallel)
fast-ebook scan library/ --workers 8
fast-ebook scan library/ --format csv > catalog.csv
```

Install via GitHub Releases or build from source: `cargo build -p fast-ebook-cli --release`

## Item Type Constants

| Constant | Value |
|----------|-------|
| `ITEM_UNKNOWN` | 0 |
| `ITEM_IMAGE` | 1 |
| `ITEM_STYLE` | 2 |
| `ITEM_SCRIPT` | 3 |
| `ITEM_NAVIGATION` | 4 |
| `ITEM_VECTOR` | 5 |
| `ITEM_FONT` | 6 |
| `ITEM_VIDEO` | 7 |
| `ITEM_AUDIO` | 8 |
| `ITEM_DOCUMENT` | 9 |
| `ITEM_COVER` | 10 |
| `ITEM_SMIL` | 11 |

## License

MIT
