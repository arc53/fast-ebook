#!/usr/bin/env python3
"""Fetch the W3C epub-tests suite and repackage each test into a .epub file.

The w3c/epub-tests repository stores test packages unzipped (one directory
per test). This script shallow-clones the repo and zips each test directory
into a valid .epub (mimetype first, stored uncompressed, per the OCF spec).

Usage:
    python tests/fetch_w3c_tests.py            # use existing clone if present
    python tests/fetch_w3c_tests.py --refresh  # re-clone from scratch

The resulting .epub files land in tests/fixtures/w3c/ and are consumed by
tests/test_w3c_conformance.py as parser stress fixtures.
"""

import argparse
import shutil
import subprocess
import sys
import zipfile
from pathlib import Path

REPO_URL = "https://github.com/w3c/epub-tests.git"
# Pinned upstream commit. Bump together with tests/w3c_baseline.json after
# re-running tests/baseline_w3c_epubcheck.py against the new tree.
PINNED_COMMIT = "45feac979d9b12b502f124db7bc5056977628417"

HERE = Path(__file__).parent
CACHE_DIR = HERE / "fixtures" / "w3c"
CLONE_DIR = CACHE_DIR / "_repo"


def clone_repo(refresh: bool = False) -> None:
    if CLONE_DIR.exists():
        if refresh:
            shutil.rmtree(CLONE_DIR)
        else:
            print(f"Using existing clone at {CLONE_DIR}")
            return
    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    print(f"Cloning {REPO_URL} @ {PINNED_COMMIT[:8]}...")
    subprocess.check_call(
        ["git", "clone", "--filter=blob:none", REPO_URL, str(CLONE_DIR)]
    )
    subprocess.check_call(
        ["git", "-C", str(CLONE_DIR), "checkout", PINNED_COMMIT]
    )


def package_test(src_dir: Path, dst: Path) -> None:
    """Zip an unpacked EPUB directory into a valid .epub file.

    Per OCF: the `mimetype` entry must be first, stored uncompressed, and
    must contain no extra fields. Everything else is DEFLATEd.
    """
    with zipfile.ZipFile(dst, "w", zipfile.ZIP_DEFLATED) as zf:
        mimetype_path = src_dir / "mimetype"
        if mimetype_path.exists():
            zf.write(
                mimetype_path, "mimetype", compress_type=zipfile.ZIP_STORED
            )
        for path in sorted(src_dir.rglob("*")):
            if path.is_dir():
                continue
            rel = path.relative_to(src_dir).as_posix()
            if rel == "mimetype" or path.name == ".DS_Store":
                continue
            zf.write(path, rel)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--refresh", action="store_true", help="Re-clone the upstream repo"
    )
    args = parser.parse_args()

    clone_repo(refresh=args.refresh)

    tests_dir = CLONE_DIR / "tests"
    if not tests_dir.exists():
        print(f"ERROR: {tests_dir} not found", file=sys.stderr)
        return 1

    count = 0
    for test_dir in sorted(tests_dir.iterdir()):
        if not test_dir.is_dir() or not (test_dir / "mimetype").exists():
            continue
        dst = CACHE_DIR / f"{test_dir.name}.epub"
        package_test(test_dir, dst)
        count += 1

    print(f"Packaged {count} W3C test EPUBs into {CACHE_DIR}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
