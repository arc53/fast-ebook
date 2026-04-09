# Threat Model

## System Description

fast-ebook is a Rust+Python library and CLI for reading, writing, validating, and converting EPUB files. It processes untrusted ZIP archives containing XML and XHTML documents.

## Trust Boundaries

```
┌───────────────────────────────────────────────┐
│ Untrusted Input                               │
│ ┌─────────┐ ┌─────────┐ ┌─────────────────┐  │
│ │ EPUB    │ │ Python  │ │ CLI arguments   │  │
│ │ files   │ │ API     │ │ (paths, flags)  │  │
│ │ (ZIP)   │ │ callers │ │                 │  │
│ └────┬────┘ └────┬────┘ └───────┬─────────┘  │
│      │           │              │             │
├──────┼───────────┼──────────────┼─────────────┤
│      ▼           ▼              ▼             │
│ ┌─────────────────────────────────────────┐   │
│ │ PyO3 Boundary / CLI Argument Parser     │   │
│ └──────────────────┬──────────────────────┘   │
│                    ▼                          │
│ ┌─────────────────────────────────────────┐   │
│ │ Rust Core                               │   │
│ │  reader.rs → ZIP extraction → XML parse │   │
│ │  writer.rs → XML generation → ZIP write │   │
│ │  markdown.rs → HTML tag stripping       │   │
│ └──────────────────┬──────────────────────┘   │
│                    ▼                          │
│ ┌─────────────────────────────────────────┐   │
│ │ Trusted Output                          │   │
│ │  EpubBook structs, Markdown text,       │   │
│ │  EPUB files, extracted files            │   │
│ └─────────────────────────────────────────┘   │
└───────────────────────────────────────────────┘
```

## Threat Actors

| Actor | Capability | Motivation |
|-------|-----------|------------|
| **Malicious EPUB author** | Crafts EPUB files with malformed ZIP, XML, or paths | File exfiltration, DoS, code execution via downstream renderers |
| **Untrusted Python caller** | Passes arbitrary strings to API (metadata, filenames, content) | XML injection, path traversal, DoS |
| **Supply chain attacker** | Compromises a dependency or CI artifact | Code execution in builds or end-user systems |

## Attack Surfaces

### 1. EPUB File Parsing (reader.rs, container.rs, metadata.rs, manifest.rs, spine.rs, ncx.rs, nav.rs)

**Input:** Untrusted ZIP archive from any source.

| Threat | Mitigation | Status |
|--------|-----------|--------|
| **ZIP bomb** (tiny compressed → huge decompressed) | 100MB max per entry (`MAX_ENTRY_SIZE`), checked before allocation | **Mitigated** |
| **ZIP slip** (path traversal in entry names) | Reader extracts content by manifest href into memory, never writes to disk. CLI extract sanitizes paths (rejects `..`, absolute paths, checks canonical prefix) | **Mitigated** |
| **XXE** (XML external entity injection) | roxmltree does not process DTDs or external entities by design | **Mitigated by dependency** |
| **Billion laughs** (exponential entity expansion) | roxmltree does not expand internal entities | **Mitigated by dependency** |
| **Stack overflow via deep nesting** | NCX and Nav parsers capped at 100 levels (`MAX_TOC_DEPTH`, `MAX_NAV_DEPTH`) | **Mitigated** |
| **Integer overflow on file size** | Size checked against MAX_ENTRY_SIZE (fits in usize on all platforms) before cast | **Mitigated** |
| **Malformed XML** | roxmltree returns parse errors; reader surfaces them as `EpubError::InvalidOpf` | **Mitigated** |
| **Missing OPF sections** | Reader tolerates missing `<metadata>`, `<manifest>`, `<spine>` — returns empty defaults | **Mitigated** |

### 2. EPUB Writing (writer.rs)

**Input:** EpubBook populated via Python API.

| Threat | Mitigation | Status |
|--------|-----------|--------|
| **XML injection via metadata values** | All values passed through `xml_escape_into()` (single-pass, escapes `& < > " '`) | **Mitigated** |
| **XML injection via metadata field names** | `is_safe_xml_name()` rejects names containing `< > " ' /` or other special chars | **Mitigated** |
| **Invalid EPUB output** | Validated by EPUBCheck in CI (5 tests covering all writer code paths) | **Mitigated** |

### 3. CLI File Operations (cli/src/main.rs)

**Input:** File paths from command-line arguments, item hrefs from EPUB.

| Threat | Mitigation | Status |
|--------|-----------|--------|
| **Path traversal in extract** | Rejects `..` components and absolute paths; verifies resolved path stays within canonical output dir | **Mitigated** |
| **Symlink escape in extract** | `canonicalize()` resolves symlinks before prefix check; `create_dir_all` could follow symlinks in parent creation | **Partially mitigated** — low practical risk since attacker would need write access to the output directory |

### 4. Python API (epub.py, pybridge.rs)

**Input:** Arbitrary Python objects and strings from calling code.

| Threat | Mitigation | Status |
|--------|-----------|--------|
| **Path injection in read_epub/write_epub** | Paths resolved via `Path.resolve()` before passing to Rust | **Mitigated** |
| **Arbitrary metadata injection** | Field names validated by `is_safe_xml_name()`, values escaped by `xml_escape_into()` | **Mitigated** |
| **Memory exhaustion via huge content** | Content passed as bytes; Python's own memory limits apply | **Acceptable** |

### 5. Markdown Conversion (markdown.rs)

**Input:** XHTML content from EPUB items.

| Threat | Mitigation | Status |
|--------|-----------|--------|
| **XSS via markdown output** | Markdown converter strips HTML tags; output is plain text with markdown formatting. If downstream renders markdown→HTML, standard markdown rendering sanitization applies | **Acceptable** — not our responsibility to sanitize downstream renderers |
| **`javascript:` URLs in links** | Links extracted as-is from `<a href>` attributes and preserved in markdown | **Acceptable** — markdown renderers should handle URL sanitization |

### 6. Supply Chain (Cargo.toml, ci.yml)

| Threat | Mitigation | Status |
|--------|-----------|--------|
| **Malicious dependency** | All 42 transitive deps are MIT/Apache-2.0 permissive; `cargo license` checked | **Monitored** |
| **CI artifact tampering** | EPUBCheck downloaded from GitHub over HTTPS without checksum verification | **Residual risk** — should add SHA256 verification |

## Residual Risks (accepted)

| Risk | Severity | Justification |
|------|----------|--------------|
| `is_safe_xml_name()` allows Unicode letters | Low | Valid per XML 1.0 spec; no known parser exploits via Unicode element names |
| `file_name` param in Python not validated for `..` | Low | Writer stores href as-is in the EPUB; no disk write at construction time. The resulting EPUB would have a weird href but no file system impact. CLI extract already sanitizes on the read-back side |
| `expect()` panic in `batch.rs` thread pool creation | Low | Only fails if OS cannot create threads (out-of-memory); process is already in trouble at that point |
| Markdown `extract_attr()` is fragile | Low | Only affects link text in markdown output; does not affect EPUB integrity or enable code execution |
| No manifest item count limit | Low | Bounded by ZIP entry size limit (100MB) and available memory |

---

## Security Controls Summary

| Control | Implementation |
|---------|---------------|
| ZIP entry size limit | 100MB per entry (`reader.rs`, `model.rs`) |
| Path traversal prevention | Reject `..` + canonical prefix check (`cli/main.rs`) |
| XML value escaping | Single-pass `xml_escape_into()` (`writer.rs`) |
| XML name validation | `is_safe_xml_name()` (`writer.rs`) |
| Recursion depth limit | 100 levels for NCX and Nav (`ncx.rs`, `nav.rs`) |
| No XXE/entity expansion | roxmltree by design |
| EPUBCheck validation | 5 CI tests against W3C validator |
| Dependency license audit | `cargo license` — all MIT-compatible |
| Security regression tests | 12 tests in `test_security.py` |
