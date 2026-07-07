from __future__ import annotations

import json
from pathlib import Path

from ares_netguard.features.evidence_windows import (
    FEATURE_ROW_SCHEMA_VERSION,
    REPORT_SCHEMA_VERSION,
    dump_feature_window_report,
    generate_feature_window_report,
    load_synthetic_telemetry_events,
)


def test_fixture_generates_synthetic_telemetry_feature_windows(tmp_path: Path) -> None:
    fixture = Path("tests/fixtures/telemetry_foundation/synthetic_events.jsonl")
    output = tmp_path / "telemetry-feature-windows.json"

    report = generate_feature_window_report(load_synthetic_telemetry_events(fixture))
    dump_feature_window_report(report, output)
    persisted = json.loads(output.read_text(encoding="utf-8"))
    rendered = json.dumps(persisted, sort_keys=True)

    assert persisted["schema_version"] == REPORT_SCHEMA_VERSION
    assert persisted["feature_row_schema"] == FEATURE_ROW_SCHEMA_VERSION
    assert persisted["window_sizes_minutes"] == [1, 5]
    assert persisted["row_count"] == 5
    assert all(row["schema_version"] == FEATURE_ROW_SCHEMA_VERSION for row in persisted["rows"])
    assert any(row["entity_id"] == "host-alpha" for row in persisted["rows"])
    assert any(row["features"]["window_size_minutes"] == 5 for row in persisted["rows"])
    assert '"live_capture_enabled": false' in rendered
    assert '"pcap_parsing_enabled": false' in rendered
    for forbidden in ("192.168.", "example.com", "http://", "/home/", ".pcap", "password"):
        assert forbidden not in rendered
