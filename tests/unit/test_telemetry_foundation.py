from __future__ import annotations

import json
from pathlib import Path

import pytest

from ares_netguard.features.evidence_windows import (
    FEATURE_ROW_SCHEMA_VERSION,
    INPUT_SCHEMA_VERSION,
    REPORT_SCHEMA_VERSION,
    dump_feature_window_report,
    generate_feature_window_report,
    load_synthetic_telemetry_events,
    normalize_telemetry_event,
    validate_feature_window_report,
)


def _event(**updates: object) -> dict[str, object]:
    row: dict[str, object] = {
        "schema_version": INPUT_SCHEMA_VERSION,
        "source_kind": "zeek_conn",
        "entity_id": "host-alpha",
        "timestamp": "2026-01-01T00:00:15Z",
        "event_count": 1,
        "connection_count": 1,
        "dns_query_count": 0,
        "dns_failure_count": 0,
        "alert_severity": 0,
        "bytes_in": 1200,
        "bytes_out": 5400,
        "duration_ms": 3200,
        "destination_asset_id": "asset-edge",
        "service_name": "web",
        "tls_unknown": False,
        "runtime_event_count": 0,
    }
    row.update(updates)
    return row


def test_generates_deterministic_feature_windows() -> None:
    events = [
        normalize_telemetry_event(_event(), event_index=0),
        normalize_telemetry_event(
            _event(
                source_kind="zeek_dns",
                timestamp="2026-01-01T00:00:45Z",
                connection_count=0,
                dns_query_count=1,
                dns_failure_count=1,
                bytes_in=80,
                bytes_out=90,
                duration_ms=40,
                destination_asset_id="asset-resolver",
                service_name="dns",
            ),
            event_index=1,
        ),
        normalize_telemetry_event(
            _event(
                source_kind="suricata_alert",
                timestamp="2026-01-01T00:04:10Z",
                connection_count=0,
                alert_severity=4,
                bytes_in=0,
                bytes_out=0,
                duration_ms=0,
                service_name="alert",
            ),
            event_index=2,
        ),
    ]

    report = generate_feature_window_report(events)

    validate_feature_window_report(report)
    assert report["schema_version"] == REPORT_SCHEMA_VERSION
    assert report["feature_row_schema"] == FEATURE_ROW_SCHEMA_VERSION
    assert report["window_sizes_minutes"] == [1, 5]
    assert report["row_count"] == 3

    five_minute_rows = [
        row for row in report["rows"] if row["features"]["window_size_minutes"] == 5
    ]
    assert len(five_minute_rows) == 1
    features = five_minute_rows[0]["features"]
    assert features["event_count"] == 3.0
    assert features["connection_count"] == 1.0
    assert features["dns_query_count"] == 1.0
    assert features["dns_failure_ratio"] == 1.0
    assert features["alert_severity_sum"] == 4.0
    assert features["max_alert_severity"] == 4.0
    assert features["bytes_in_total"] == 1280.0
    assert features["bytes_out_total"] == 5490.0
    assert features["destination_diversity"] == 2.0
    assert features["service_diversity"] == 3.0


def test_fixture_loader_uses_strict_json_and_normalizes_events(tmp_path: Path) -> None:
    path = tmp_path / "events.jsonl"
    path.write_text(json.dumps(_event(), sort_keys=True) + "\n", encoding="utf-8")

    events = load_synthetic_telemetry_events(path)

    assert events[0]["schema_version"] == "telemetry_event.v0"
    assert events[0]["event_id"] == "telemetry-event-0001"
    assert events[0]["minute_start"] == "2026-01-01T00:00:00Z"


def test_strict_json_loader_rejects_nan(tmp_path: Path) -> None:
    path = tmp_path / "bad.jsonl"
    path.write_text(
        '{"schema_version":"synthetic_telemetry_event.v0","bytes_in":NaN}\n',
        encoding="utf-8",
    )

    with pytest.raises(ValueError, match="non-strict JSON constant"):
        load_synthetic_telemetry_events(path)


@pytest.mark.parametrize(
    ("field", "value"),
    [
        ("destination_asset_id", "192.168.1.10"),
        ("destination_asset_id", "example.com"),
        ("service_name", "secret_web"),
        ("service_name", "/home/user/report"),
    ],
)
def test_privacy_guard_rejects_raw_identifiers(field: str, value: str) -> None:
    with pytest.raises(ValueError, match="unsafe raw identifier|synthetic/coarse|sanitized"):
        normalize_telemetry_event(_event(**{field: value}), event_index=0)


def test_rejects_impossible_dns_failure_counts() -> None:
    with pytest.raises(ValueError, match="dns_failure_count cannot exceed dns_query_count"):
        normalize_telemetry_event(
            _event(dns_query_count=1, dns_failure_count=2),
            event_index=0,
        )


def test_report_validation_catches_tampered_counts() -> None:
    report = generate_feature_window_report([normalize_telemetry_event(_event(), event_index=0)])
    report["row_count"] = 99

    with pytest.raises(ValueError, match="row_count"):
        validate_feature_window_report(report)


def test_report_output_does_not_copy_private_or_artifact_strings() -> None:
    report = generate_feature_window_report([normalize_telemetry_event(_event(), event_index=0)])
    rendered = json.dumps(report, sort_keys=True)

    for forbidden in ("192.168.", "example.com", "http://", "/home/", ".pcap", "password"):
        assert forbidden not in rendered


def test_dump_rejects_repository_root_outputs(tmp_path: Path) -> None:
    report = generate_feature_window_report([normalize_telemetry_event(_event(), event_index=0)])
    output_path = Path("telemetry-feature-report.json")

    with pytest.raises(ValueError, match="ignored runtime roots"):
        dump_feature_window_report(report, output_path)

    allowed_output = tmp_path / "telemetry-feature-report.json"
    dump_feature_window_report(report, allowed_output)
    assert json.loads(allowed_output.read_text(encoding="utf-8"))["schema_version"] == (
        REPORT_SCHEMA_VERSION
    )
