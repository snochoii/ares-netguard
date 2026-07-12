from __future__ import annotations

import json
from pathlib import Path

from ares_netguard.investigation.agentic_layer import generate_investigation_report
from ares_netguard.models.disagreement import generate_disagreement_report
from ares_netguard.models.evaluation_bundle import generate_evaluation_bundle
from ares_netguard.models.registry_metadata import generate_registry_metadata
from ares_netguard.models.score_row_composer import compose_score_rows, load_residual_report
from ares_netguard.models.time_series_residual import (
    MODEL_ID,
    REPORT_SCHEMA_VERSION,
    dump_report,
    generate_residual_report,
    load_time_window_rows,
)
from ares_netguard.storage.evidence_index import generate_evidence_index


def test_fixture_flows_through_v1_consumers_without_changing_consumer_schemas(
    tmp_path: Path,
) -> None:
    fixture = Path("tests/fixtures/time_series_residual/synthetic_windows.jsonl")
    output = tmp_path / "time-series-residual-report.json"

    residual_report = generate_residual_report(load_time_window_rows(fixture))
    dump_report(residual_report, output)
    persisted = load_residual_report(output)

    assert persisted["schema_version"] == REPORT_SCHEMA_VERSION
    assert persisted["forecast_backend"]["backend_id"] == "rolling_mean_proxy_v1"
    assert persisted["calibration"]["count"] == 8
    assert len(persisted["rows"]) == 3
    assert {row["window_start"] for row in persisted["rows"]} == {"2026-01-01T00:15:00Z"}
    assert persisted == json.loads(output.read_text(encoding="utf-8"))

    score_rows = compose_score_rows(residual_reports=[persisted])
    assert {row["schema_version"] for row in score_rows} == {"model_score_row.v0"}
    assert all(MODEL_ID in row["scores"] for row in score_rows)
    assert all(
        row["scores"][MODEL_ID]["evidence"][-1]["evidence_kind"]
        == "forecast_backend_calibration_provenance"
        for row in score_rows
    )

    disagreement_report = generate_disagreement_report(score_rows)
    assert disagreement_report["schema_version"] == "model_disagreement_report.v0"
    assert MODEL_ID in disagreement_report["evidence_by_model"]

    investigation_report = generate_investigation_report(
        disagreement_report,
        evidence_reports=[persisted],
    )
    assert investigation_report["schema_version"] == "agentic_investigation_report.v0"
    assert investigation_report["evidence_report_schemas"] == [REPORT_SCHEMA_VERSION]

    evaluation = generate_evaluation_bundle(
        [persisted, score_rows, disagreement_report, investigation_report]
    )
    assert evaluation["schema_version"] == "model_evaluation_bundle.v0"
    assert REPORT_SCHEMA_VERSION in evaluation["aggregate_summary"]["schemas_present"]

    registry = generate_registry_metadata(evaluation)
    assert registry["schema_version"] == "model_registry_metadata.v0"
    residual_entry = next(entry for entry in registry["entries"] if entry["model_id"] == MODEL_ID)
    assert REPORT_SCHEMA_VERSION in residual_entry["observed_source_schemas"]

    index = generate_evidence_index(
        [persisted, score_rows, disagreement_report, investigation_report, registry]
    )
    assert index["schema_version"] == "evidence_index.v0"
    assert REPORT_SCHEMA_VERSION in index["aggregate_summary"]["schemas_present"]
