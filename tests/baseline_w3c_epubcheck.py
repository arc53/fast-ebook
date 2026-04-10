#!/usr/bin/env python3
"""Baseline-validate every W3C epub-tests fixture with EPUBCheck.

The W3C epub-tests corpus is heterogeneous: most packages are spec-valid
EPUBs that exercise particular reading-system features, but some are
intentionally malformed (negative tests) or use draft features that
EPUBCheck flags as errors.

To make roundtrip testing meaningful we first need to know which inputs
EPUBCheck *itself* accepts. This script runs EPUBCheck against every
fixture once and writes the result to:

    tests/fixtures/w3c/baseline.json

The roundtrip test (tests/test_epubcheck_w3c.py) then only validates
fast-ebook's output against the inputs that the validator already accepts.
That gives a precise no-regression invariant: "if EPUBCheck accepted the
input, it must also accept fast-ebook's roundtripped output."

Requires Java + epubcheck.jar (set EPUBCHECK_JAR or place epubcheck.jar in
the project root).

Usage:
    EPUBCHECK_JAR=/path/to/epubcheck.jar python tests/baseline_w3c_epubcheck.py
    python tests/baseline_w3c_epubcheck.py --workers 8
"""

import argparse
import json
import os
import shutil
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path

HERE = Path(__file__).parent
W3C_DIR = HERE / "fixtures" / "w3c"
BASELINE_PATH = HERE / "w3c_baseline.json"

EPUBCHECK_JAR = os.environ.get("EPUBCHECK_JAR", "epubcheck.jar")


def run_epubcheck(epub_path: Path) -> tuple[bool, str]:
    """Validate one EPUB. Returns (passed, message)."""
    try:
        result = subprocess.run(
            ["java", "-jar", EPUBCHECK_JAR, str(epub_path)],
            capture_output=True,
            text=True,
            timeout=60,
        )
    except subprocess.TimeoutExpired:
        return False, "EPUBCheck timed out"
    output = (result.stdout + result.stderr).strip()
    return result.returncode == 0, output


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--workers",
        type=int,
        default=4,
        help="Parallel EPUBCheck invocations (default: 4)",
    )
    args = parser.parse_args()

    if not shutil.which("java"):
        print("ERROR: java not found in PATH", file=sys.stderr)
        return 1
    if not Path(EPUBCHECK_JAR).is_file():
        print(
            f"ERROR: EPUBCHECK_JAR not found: {EPUBCHECK_JAR}",
            file=sys.stderr,
        )
        return 1
    if not W3C_DIR.exists():
        print(
            f"ERROR: {W3C_DIR} not found. Run tests/fetch_w3c_tests.py first.",
            file=sys.stderr,
        )
        return 1

    fixtures = sorted(W3C_DIR.glob("*.epub"))
    if not fixtures:
        print(f"ERROR: no .epub files in {W3C_DIR}", file=sys.stderr)
        return 1

    print(f"Baselining {len(fixtures)} fixtures with EPUBCheck (workers={args.workers})...")
    start = time.time()

    passing: list[str] = []
    failing: dict[str, str] = {}

    with ThreadPoolExecutor(max_workers=args.workers) as pool:
        futures = {pool.submit(run_epubcheck, f): f for f in fixtures}
        for i, fut in enumerate(as_completed(futures), 1):
            f = futures[fut]
            ok, output = fut.result()
            if ok:
                passing.append(f.name)
            else:
                # Keep only the summary line(s) to keep baseline.json small.
                msg_lines = [
                    ln
                    for ln in output.splitlines()
                    if "ERROR" in ln or "FATAL" in ln or "Messages:" in ln
                ]
                failing[f.name] = "\n".join(msg_lines[:5]) or output[:300]
            if i % 20 == 0 or i == len(fixtures):
                elapsed = time.time() - start
                print(
                    f"  {i}/{len(fixtures)}  "
                    f"({len(passing)} pass, {len(failing)} fail)  "
                    f"[{elapsed:.1f}s]"
                )

    passing.sort()
    baseline = {
        "epubcheck_jar": str(Path(EPUBCHECK_JAR).resolve()),
        "fixture_count": len(fixtures),
        "pass_count": len(passing),
        "fail_count": len(failing),
        "passing": passing,
        "failing": failing,
    }
    BASELINE_PATH.write_text(json.dumps(baseline, indent=2, sort_keys=True))

    elapsed = time.time() - start
    print()
    print(f"Wrote {BASELINE_PATH}")
    print(
        f"  passing: {len(passing)}/{len(fixtures)}  "
        f"failing: {len(failing)}/{len(fixtures)}  "
        f"({elapsed:.1f}s)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
