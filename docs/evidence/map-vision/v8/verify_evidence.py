#!/usr/bin/env python3
"""Verify the V8 evidence: manifest hashes/dimensions and the 32 blind reads.

Read-only. Exits non-zero on any mismatch.
"""

from __future__ import annotations

import hashlib
import json
import struct
from pathlib import Path

ROOT = Path(__file__).resolve().parent
STATES = [
    "application", "library", "no-entry", "empty",
    "all-test", "capped", "partial-parse", "single-file",
]


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def png_size(path: Path) -> tuple[int, int]:
    head = path.read_bytes()[:24]
    assert head[:8] == b"\x89PNG\r\n\x1a\n", f"not a PNG: {path}"
    return struct.unpack(">II", head[16:24])


def main() -> None:
    manifest = json.loads((ROOT / "manifest.json").read_text())
    reads = json.loads((ROOT / "reads.json").read_text())
    assert [s["state"] for s in manifest["states"]] == STATES, "manifest state order"
    assert reads["states"] == STATES, "reads state order"
    a1 = strip = 0
    for state in STATES:
        entry = next(s for s in manifest["states"] if s["state"] == state)
        final = ROOT / entry["final"]["file"]
        strip_png = ROOT / entry["strip"]["file"]
        html = ROOT / entry["html"]
        assert final.exists() and strip_png.exists() and html.exists(), state
        assert sha256(final) == entry["final"]["sha256"], f"{state} final hash"
        assert sha256(strip_png) == entry["strip"]["sha256"], f"{state} strip hash"
        assert sha256(html) == entry["html_sha256"], f"{state} html hash"
        assert png_size(final) == (1440, 900), f"{state} final dimensions"
        assert png_size(strip_png) == (entry["strip"]["width"], entry["strip"]["height"]), f"{state} strip dimensions"
        assert '<section id="map">' in html.read_text(), f"{state} missing #map"
        img = reads["images"][state]
        assert img["sha256"] == entry["final"]["sha256"], f"{state} reads final hash"
        assert img["strip_sha256"] == entry["strip"]["sha256"], f"{state} reads strip hash"
        assert img["html_sha256"] == entry["html_sha256"], f"{state} reads html hash"
        a1_reads = reads["reads"][state]
        strip_reads = reads["strip_reads"][state]
        assert [r["read"] for r in a1_reads] == ["A", "B"], f"{state} A1 reads"
        assert [r["read"] for r in strip_reads] == ["A", "B"], f"{state} strip reads"
        for r in a1_reads:
            assert r["image_sha256"] == entry["final"]["sha256"], f"{state} A1 image hash"
            assert r["question_ok"] and r["action_ok"] and r["pass"], f"{state} A1 pass"
            a1 += 1
        for r in strip_reads:
            assert r["image_sha256"] == entry["strip"]["sha256"], f"{state} strip image hash"
            assert r["mass_ok"] and r["action_ok"] and r["pass"], f"{state} strip pass"
            strip += 1
    assert a1 == 16 and strip == 16, (a1, strip)
    print(f"evidence hashes ok · {len(STATES)} states · {a1} A1 reads · {strip} strip reads")


if __name__ == "__main__":
    main()
