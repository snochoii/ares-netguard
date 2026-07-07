"""CLI wrapper for the synthetic telemetry foundation v0 pipeline.

This module intentionally handles only caller-provided synthetic fixture rows.
It does not perform live capture, PCAP parsing, directory discovery, external
enrichment, or private telemetry collection.
"""

from __future__ import annotations

from collections.abc import Sequence

from ares_netguard.features.evidence_windows import main as evidence_windows_main


def main(argv: Sequence[str] | None = None) -> int:
    return evidence_windows_main(argv)


if __name__ == "__main__":
    raise SystemExit(main())
