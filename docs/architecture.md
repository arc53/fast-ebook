# Architecture

## Overview

fast-ebook is a Rust-powered Python library for EPUB2/EPUB3 manipulation. The Rust core handles all parsing, writing, validation, and markdown conversion. Python receives pre-built objects across the PyO3 boundary with minimal overhead.

```
┌─────────────────────────────────────────────────────┐
│  Python Layer (python/fast_ebook/)                  │
│  ┌──────────┐ ┌──────────┐ ┌───────────┐           │
│  │ epub.py  │ │__init__.py│ │compat/    │           │
│  │EpubBook  │ │ITEM_*    │ │ drop-in   │           │
│  │EpubHtml  │ │constants │ │ compat    │           │
│  │Link, etc.│ │          │ │ shim      │           │
│  └────┬─────┘ └──────────┘ └───────────┘           │
│       │ PyO3 boundary                               │
├───────┼─────────────────────────────────────────────┤
│  Rust Core (src/)                                   │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐            │
│  │pybridge  │ │ reader   │ │ writer   │            │
│  │PyEpubBook│ │read_epub │ │write_epub│            │
│  │PyEpubItem│ │  inner() │ │  inner() │            │
│  └────┬─────┘ └────┬─────┘ └────┬─────┘            │
│       │             │            │                   │
│  ┌────┴─────────────┴────────────┴──────┐           │
│  │              model.rs                │           │
│  │  EpubBook, EpubItem (Arc, OnceLock)  │           │
│  └──────────────────────────────────────┘           │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐            │
│  │container │ │metadata  │ │manifest  │            │
│  │  .rs     │ │  .rs     │ │  .rs     │            │
│  └──────────┘ └──────────┘ └──────────┘            │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐            │
│  │ spine.rs │ │  ncx.rs  │ │  nav.rs  │            │
│  └──────────┘ └──────────┘ └──────────┘            │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐            │
│  │validation│ │ batch.rs │ │markdown  │            │
│  │  .rs     │ │ (rayon)  │ │  .rs     │            │
│  └──────────┘ └──────────┘ └──────────┘            │
├─────────────────────────────────────────────────────┤
│  CLI Binary (cli/)                                  │
│  Uses the Rust core without Python (no PyO3)        │
│  Commands: validate, info, extract, convert, scan   │
└─────────────────────────────────────────────────────┘
```

## Project Structure

```
fast-ebook/
├── src/                        # Rust core library
│   ├── lib.rs                  # Module declarations, PyO3 module registration
│   ├── model.rs                # EpubBook, EpubItem (Arc-wrapped, OnceLock lazy loading)
│   ├── reader.rs               # EPUB reading (generic over Read+Seek)
│   ├── writer.rs               # EPUB writing (generic over Write+Seek)
│   ├── pybridge.rs             # PyO3 class wrappers (#[cfg(feature = "python")])
│   ├── container.rs            # META-INF/container.xml parsing
│   ├── metadata.rs             # Dublin Core metadata extraction
│   ├── manifest.rs             # OPF <manifest> parsing
│   ├── spine.rs                # OPF <spine> parsing
│   ├── ncx.rs                  # EPUB2 NCX table of contents
│   ├── nav.rs                  # EPUB3 Nav document parsing
│   ├── item_type.rs            # ITEM_* enum + media-type mapping
│   ├── validation.rs           # EPUB spec validation
│   ├── batch.rs                # Parallel batch reading (rayon)
│   ├── markdown.rs             # HTML-to-Markdown conversion
│   └── errors.rs               # Error types (EpubError → PyErr)
├── cli/                        # Standalone CLI binary
│   ├── Cargo.toml              # depends on fast-ebook with default-features=false
│   └── src/main.rs             # clap-based CLI (validate/info/extract/convert/scan)
├── python/fast_ebook/          # Python package
│   ├── __init__.py             # ITEM_* constants, re-exports
│   ├── epub.py                 # EpubBook subclass, convenience classes, read/write/open
│   ├── compat/                 # Drop-in compatibility layer
│   ├── *.pyi                   # Type stubs
│   └── py.typed                # PEP 561 marker
├── tests/                      # Python test suite (109 tests)
├── bench/                      # Performance benchmarks
├── Cargo.toml                  # Workspace root + PyO3 cdylib/rlib
└── pyproject.toml              # maturin build config
```

## Key Design Decisions

### 1. PyO3 cdylib at repo root

Maturin works best when the PyO3 crate is at the repo root. The CLI is a separate workspace member (`cli/`) that depends on the same crate with `default-features = false` to exclude PyO3.

### 2. Feature-gated Python bindings

All PyO3 code is behind `#[cfg(feature = "python")]`. The `python` feature is enabled by default for maturin builds but disabled for the CLI binary. This allows the same Rust codebase to serve both Python and standalone binary use cases.

```toml
# Cargo.toml
[features]
default = ["python"]
python = ["pyo3"]
```

### 3. Arc-wrapped items with OnceLock lazy loading

```rust
pub struct EpubItem {
    pub id: String,
    pub href: String,
    pub media_type: String,
    pub item_type: ItemType,
    content: OnceLock<Vec<u8>>,           // Thread-safe lazy init
    pub(crate) lazy_source: Option<LazySource>,  // ZIP data + path for deferred loading
}
```

- Items are wrapped in `Arc<EpubItem>` so Python's `PyEpubItem` can hold cheap references without cloning content bytes
- `OnceLock` enables lazy loading: when `lazy=True`, content is not extracted from the ZIP until first `get_content()` call
- Thread-safe: multiple Python threads can call `get_content()` concurrently

### 4. Generic Read/Write over I/O traits

Both `reader.rs` and `writer.rs` are generic over `Read + Seek` / `Write + Seek`:

```rust
fn read_epub_inner<R: Read + Seek>(archive: &mut ZipArchive<R>, ...) -> Result<EpubBook, EpubError>
fn write_epub_inner<W: Write + Seek>(writer: W, book: &EpubBook) -> Result<W, EpubError>
```

This enables file path, `BytesIO`, and in-memory `bytes` input/output with zero code duplication.

### 5. Python subclass pattern

`PyEpubBook` uses `#[pyclass(subclass)]` so the Python `EpubBook` class can subclass it. This allows Python-level property setters (toc, spine) that normalize mixed types (Link, Section, tuples, strings) into flat Rust structures before crossing the boundary.

```
Python: epub.EpubBook (subclass)
  ├── toc setter → normalize Link/Section → _set_toc_from_entries() → Rust
  ├── spine setter → normalize str/item → _set_spine_from_entries() → Rust
  └── add_item() → convert EpubHtml/Image → add_item_raw() → Rust

Rust: PyEpubBook (#[pyclass(subclass)])
  ├── get_items(), get_metadata(), validate(), to_markdown() → pure Rust
  └── inner: EpubBook (the actual data)
```

### 6. Zero-allocation XML generation

The writer generates OPF, NCX, and Nav XML using direct string buffer operations instead of `format!()`:

```rust
// Instead of: push_str(&format!("<dc:title>{}</dc:title>", xml_escape(v)))
// We do:
opf.push_str("<dc:title>");
xml_escape_into(&mut opf, v);      // single-pass, zero temp allocations
opf.push_str("</dc:title>\n");
```

`xml_escape_into()` writes escaped chars directly into the output buffer — no intermediate `String` allocations.

## Data Flow

### Reading an EPUB

```
File/bytes → ZipArchive
  → container.xml → OPF path
  → OPF → metadata + manifest + spine
  → NCX/Nav → table of contents
  → Items (eager: extracted now, lazy: deferred)
  → EpubBook (Rust struct)
  → PyEpubBook (PyO3 wrapper) → Python
```

### Writing an EPUB

```
Python EpubBook
  → add_item() → add_item_raw() → Rust EpubItem
  → toc/spine setters → normalized entries → Rust
  → write_epub() → validate → generate OPF/NCX/Nav XML → ZIP assembly
  → mimetype (stored, first) → container.xml → OPF → items (deflated)
  → File/BytesIO
```

### Batch parallel reading

```
Python: epub.read_epubs(paths, workers=4)
  → _read_epubs() → py.allow_threads() (GIL released)
  → Rust: rayon::par_iter() over paths
  → Each thread: read_epub_with_options()
  → Collect Vec<Result<EpubBook>>
  → GIL re-acquired, convert to Vec<PyEpubBook>
  → Python list
```

## Dependencies

| Crate | Purpose | License |
|-------|---------|---------|
| pyo3 | Python bindings (feature-gated) | Apache-2.0 OR MIT |
| zip | ZIP archive read/write | MIT |
| roxmltree | XML DOM parsing | Apache-2.0 OR MIT |
| rayon | Parallel batch processing | Apache-2.0 OR MIT |
| clap | CLI argument parsing (CLI only) | Apache-2.0 OR MIT |
| serde_json | JSON output (CLI only) | Apache-2.0 OR MIT |

All dependencies are MIT-compatible. Zero copyleft in the tree.
