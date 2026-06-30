from __future__ import annotations

import json
from pathlib import Path

import pytest

from ares_netguard.models.time_series_residual import (
    MODEL_FAMILY,
    MODEL_ID,
    REPORT_SCHEMA_VERSION,
    dump_report,
    generate_residual_report,
    residual_evidence_to_score_rows,
)


def _row(
    window_start: str,
    actual_value: object,
    *,
    entity_id: str = "host-a",
) -> dict[str, object]:
    return {
        "entity_id": entity_id,
        "feature_name": "bytes_out",
        "window_start": window_start,
        "actual_value": actual_value,
    }


def test_residual_math_is_deterministic() -> None:
    report = generate_residual_report(
        [
            _row("2026-01-01T00:00:00Z", 10),
            _row("2026-01-01T00:05:00Z", 12),
            _row("2026-01-01T00:10:00Z", 14),
            _row("2026-01-01T00:15:00Z", 18),
        ]
    )

    assert report["schema_version"] == REPORT_SCHEMA_VERSION
    assert report["model_id"] == MODEL_ID
    assert report["model_family"] == MODEL_FAMILY
    assert report["rows"] == [
        {
            "entity_id": "host-a",
            "feature_name": "bytes_out",
            "window_start": "2026-01-01T00:15:00Z",
            "actual_value": 18.0,
            "forecast_mean": 12.0,
            "forecast_lower": 8.734014,
            "forecast_upper": 15.265986,
            "residual": 6.0,
            "residual_z": 3.674235,
            "conformal_score": 0.75,
            "residual_risk": 0.918559,
            "model_id": MODEL_ID,
            "model_family": MODEL_FAMILY,
        }
    ]


def test_interval_breach_is_preserved_as_disagreement_evidence() -> None:
    report = generate_residual_report(
        [
            _row("2026-01-01T00:00:00Z", 10),
            _row("2026-01-01T00:05:00Z", 12),
            _row("2026-01-01T00:10:00Z", 14),
            _row("2026-01-01T00:15:00Z", 18),
        ]
    )

    score_rows = residual_evidence_to_score_rows(report)
    score_entry = score_rows[0]["scores"][MODEL_ID]
    evidence = score_entry["evidence"][0]

    assert score_entry["risk"] == 0.918559
    assert evidence["actual_value"] > evidence["forecast_upper"]
    assert evidence["conformal_score"] == 0.75


@pytest.mark.parametrize("bad_value", [float("nan"), float("inf"), True, "42"])
def test_invalid_actual_value_is_rejected(bad_value: object) -> None:
    with pytest.raises(ValueError, match="actual_value must be a finite number"):
        generate_residual_report([_row("2026-01-01T00:00:00Z", bad_value)])


def test_missing_required_field_is_rejected() -> None:
    row = _row("2026-01-01T00:00:00Z", 10)
    del row["feature_name"]

    with pytest.raises(ValueError, match="feature_name"):
        generate_residual_report([row])


def test_duplicate_timestamp_for_series_is_rejected() -> None:
    with pytest.raises(ValueError, match="duplicate window_start"):
        generate_residual_report(
            [
                _row("2026-01-01T00:00:00Z", 10),
                _row("2026-01-01T00:00:00Z", 12),
            ]
        )


def test_unsorted_timestamp_for_series_is_rejected() -> None:
    with pytest.raises(ValueError, match="strictly increasing"):
        generate_residual_report(
            [
                _row("2026-01-01T00:00:00Z", 10),
                _row("2026-01-01T00:10:00Z", 12),
                _row("2026-01-01T00:05:00Z", 11),
            ]
        )


def test_stable_output_order_and_strict_json(tmp_path: Path) -> None:
    rows = [
        _row("2026-01-01T00:00:00Z", 20, entity_id="host-z"),
        _row("2026-01-01T00:05:00Z", 22, entity_id="host-z"),
        _row("2026-01-01T00:10:00Z", 24, entity_id="host-z"),
        _row("2026-01-01T00:15:00Z", 28, entity_id="host-z"),
        _row("2026-01-01T00:00:00Z", 10, entity_id="host-a"),
        _row("2026-01-01T00:05:00Z", 12, entity_id="host-a"),
        _row("2026-01-01T00:10:00Z", 14, entity_id="host-a"),
        _row("2026-01-01T00:15:00Z", 18, entity_id="host-a"),
    ]

    report = generate_residual_report(rows)
    output = tmp_path / "time-series-residual-report.json"
    dump_report(report, output)
    persisted = json.loads(output.read_text(encoding="utf-8"))

    assert [row["entity_id"] for row in persisted["rows"]] == ["host-a", "host-z"]
    assert persisted == json.loads(output.read_text(encoding="utf-8"))


def test_dump_report_rejects_non_finite_output(tmp_path: Path) -> None:
    with pytest.raises(ValueError, match="Out of range float values"):
        dump_report(
            {"schema_version": REPORT_SCHEMA_VERSION, "risk": float("nan")},
            tmp_path / "bad.json",
        )
