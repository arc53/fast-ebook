# Benchmarks

All benchmarks run on Apple M-series, release build (`maturin develop --release`). Compared against ebooklib 0.20 (uses lxml C extension).

## Read Speed

| EPUB | ebooklib | fast-ebook (eager) | fast-ebook (lazy) |
|------|---------|-------------------|-------------------|
| minimal (2 items) | 0.19ms | 0.10ms (**1.9x**) | 0.07ms (**2.6x**) |
| multi-chapter (7 items) | 0.30ms | 0.14ms (**2.1x**) | 0.08ms (**3.5x**) |
| War and Peace (374 items) | 26.6ms | 14.5ms (**1.8x**) | 1.5ms (**17.9x**) |

Lazy mode defers item content extraction until first `get_content()` call, making metadata-only operations nearly instant.

## Write Speed

| Chapters | ebooklib | fast-ebook | Speedup |
|----------|---------|------------|---------|
| 5 | 1.39ms | 0.39ms | **3.6x** |
| 20 | 4.64ms | 0.89ms | **5.2x** |
| 50 | 10.41ms | 1.75ms | **6.0x** |
| 100 | 20.28ms | 3.27ms | **6.2x** |

Write speedup increases with book size due to zero-allocation XML generation.

## Batch Parallel Read

| EPUBs | ebooklib (seq) | fast-ebook (seq) | fast-ebook (4 workers) |
|-------|---------------|-----------------|----------------------|
| 4 | 1.01ms | 0.49ms (2.1x) | 0.36ms (**2.8x**) |
| 16 | 4.02ms | 1.95ms (2.1x) | 1.17ms (**3.4x**) |
| 100 | 25.06ms | 11.90ms (2.1x) | 6.20ms (**4.0x**) |

GIL is released during parallel processing. Throughput scales with core count.

## Markdown Conversion (War and Peace)

| Method | Time |
|--------|------|
| Python regex (old) | 233ms |
| Rust `to_markdown()` | **71ms** (3.3x) |
| CLI binary | **70ms** |

## Memory

| | ebooklib | fast-ebook |
|---|---------|-----------|
| Peak (multi-chapter) | 99.7 KB | 3.7 KB (**96% less**) |

## Parsing Quality

100% parity with ebooklib across all test fixtures. Verified: metadata, spine order, ToC structure, content integrity.

| Metric | Result |
|--------|--------|
| Metadata accuracy | 100% |
| Spine fidelity | 100% |
| ToC fidelity | 100% |
| Content integrity | 100% |
| Word parity (War and Peace) | 100.0% (567,484 words) |
