"""Full EPUBCheck conformance test against the W3C epub-tests corpus.

For every W3C fixture that EPUBCheck *itself* accepts as a valid input
(per tests/w3c_baseline.json, produced by tests/baseline_w3c_epubcheck.py),
this test:

  1. Reads the input with fast-ebook
  2. Writes it back out with fast-ebook
  3. Re-validates the output with EPUBCheck

The invariant is **no validity regression**: anything EPUBCheck accepts as
input must also be accepted after a fast-ebook roundtrip. A failure means
the read+write path corrupted, dropped, or mis-serialized something the
spec considers required.

Skipped automatically when EPUBCheck or the baseline is missing.
"""

import json
import os
import shutil
import subprocess
from pathlib import Path

import pytest

from fast_ebook import epub

HERE = Path(__file__).parent
W3C_DIR = HERE / "fixtures" / "w3c"
BASELINE_PATH = HERE / "w3c_baseline.json"

EPUBCHECK_JAR = os.environ.get("EPUBCHECK_JAR", "epubcheck.jar")


def _epubcheck_available() -> bool:
    return bool(shutil.which("java")) and Path(EPUBCHECK_JAR).is_file()


def _load_baseline_passing() -> list[str]:
    if not BASELINE_PATH.is_file():
        return []
    return json.loads(BASELINE_PATH.read_text()).get("passing", [])


def _run_epubcheck(epub_path: Path) -> tuple[bool, str]:
    result = subprocess.run(
        ["java", "-jar", EPUBCHECK_JAR, str(epub_path)],
        capture_output=True,
        text=True,
        timeout=60,
    )
    return result.returncode == 0, result.stdout + result.stderr


_PASSING = _load_baseline_passing()

pytestmark = [
    pytest.mark.skipif(
        not _epubcheck_available(),
        reason="EPUBCheck not available (set EPUBCHECK_JAR env var)",
    ),
    pytest.mark.skipif(
        not _PASSING,
        reason="W3C baseline not generated (run python tests/baseline_w3c_epubcheck.py)",
    ),
]


@pytest.mark.parametrize("fixture_name", _PASSING, ids=lambda n: Path(n).stem)
def test_w3c_roundtrip_validates(fixture_name: str, tmp_path) -> None:
    """fast-ebook's roundtrip output must remain EPUBCheck-clean."""
    src = W3C_DIR / fixture_name
    if not src.is_file():
        pytest.skip(f"fixture not in cache: {fixture_name}")

    book = epub.read_epub(str(src))
    out = tmp_path / fixture_name
    epub.write_epub(str(out), book)

    ok, output = _run_epubcheck(out)
    if not ok:
        lines = [
            ln
            for ln in output.splitlines()
            if "ERROR" in ln or "FATAL" in ln or "WARNING" in ln
        ]
        msg = "\n".join(lines[:20]) or output[:1000]
        pytest.fail(f"EPUBCheck rejected roundtrip of {fixture_name}:\n{msg}")


def test_w3c_baseline_nonempty() -> None:
    """Sanity check: the baseline contains a meaningful number of fixtures."""
    assert len(_PASSING) >= 100, (
        f"Expected >=100 baseline-passing W3C fixtures, found {len(_PASSING)}. "
        "Re-run python tests/baseline_w3c_epubcheck.py."
    )
