"""Parser conformance smoke test against the W3C epub-tests suite.

The W3C epub-tests repository is the official conformance corpus for the
EPUB 3.x specifications. We don't aim to *pass* every test (many are reading-
system rendering checks that don't apply to a parsing library) but we do
require that our parser either:

  1. parses the file successfully and lets the read API run without crashing,
  2. or rejects it cleanly with a ValueError (the EpubError -> PyValueError
     conversion in src/errors.rs).

Anything else — a panic, segfault, or unexpected exception type — is a
parser regression.

The fixtures are produced by tests/fetch_w3c_tests.py and cached under
tests/fixtures/w3c/. The test is automatically skipped if the cache is
empty, so the rest of the suite still runs without network access.
"""

from pathlib import Path

import pytest

from fast_ebook import epub

W3C_CACHE = Path(__file__).parent / "fixtures" / "w3c"


def _discover_epubs() -> list[Path]:
    if not W3C_CACHE.exists():
        return []
    return sorted(W3C_CACHE.glob("*.epub"))


EPUBS = _discover_epubs()

pytestmark = pytest.mark.skipif(
    not EPUBS,
    reason="W3C epub-tests cache empty (run python tests/fetch_w3c_tests.py)",
)


@pytest.mark.parametrize("path", EPUBS, ids=lambda p: p.stem)
def test_w3c_parse_smoke(path: Path) -> None:
    """The parser must not crash on any W3C test package."""
    try:
        book = epub.read_epub(str(path))
    except ValueError:
        # Clean rejection — acceptable for negative tests or features we
        # don't yet support. The contract is "no crash", not "always parse".
        return

    # Successfully parsed — exercise the read API to surface lazy crashes.
    items = list(book.get_items())
    for item in items:
        _ = item.get_content()

    # Metadata access should never raise on a parsed book.
    _ = book.get_metadata("DC", "title")
    _ = book.get_metadata("DC", "language")


def test_w3c_cache_nonempty() -> None:
    """Sanity check: at least one fixture was discovered.

    Guards against silent skips when the fetch script ran but produced
    nothing (e.g. upstream layout changed).
    """
    assert len(EPUBS) > 50, (
        f"Expected >50 W3C test fixtures, found {len(EPUBS)}. "
        "Re-run python tests/fetch_w3c_tests.py --refresh."
    )
