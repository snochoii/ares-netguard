from __future__ import annotations

import json
from pathlib import Path

from ares_netguard.models.disagreement import generate_disagreement_report
from ares_netguard.models.time_series_residual import (
    MODEL_ID,
    REPORT_SCHEMA_VERSION,
    dump_report,
    generate_residual_report,
    load_time_window_rows,
    residual_evidence_to_score_rows,
)


def test_fixture_generates_residual_report_and_disagreement_rows(tmp_path: Path) -> None:
    fixture = Path("tests/fixtures/time_series_residual/synthetic_windows.jsonl")
    output = tmp_path / "time-series-residual-report.json"

    residual_report = generate_residual_report(load_time_window_rows(fixture))
    dump_report(residual_report, output)
    persisted = json.loads(output.read_text(encoding="utf-8"))

    assert persisted["schema_version"] == REPORT_SCHEMA_VERSION
    assert persisted["rows"]
    assert persisted == json.loads(output.read_text(encoding="utf-8"))

    score_rows = residual_evidence_to_score_rows(persisted)
    assert score_rows[0]["schema_version"] == "model_score_row.v0"
    assert score_rows[0]["scores"][MODEL_ID]["risk"] == max(
        row["residual_risk"]
        for row in persisted["rows"]
        if row["entity_id"] == score_rows[0]["entity_id"]
        and row["window_start"] == score_rows[0]["window_start"]
    )

    disagreement_report = generate_disagreement_report(score_rows)
    assert MODEL_ID in disagreement_report["evidence_by_model"]
    assert disagreement_report["model_score_matrix"][0]["scores"][MODEL_ID] >= 0.0
    assert disagreement_report["row_reports"][0]["schema_version"] == "model_score_row.v0"
