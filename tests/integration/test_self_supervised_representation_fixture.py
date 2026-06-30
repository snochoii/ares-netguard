from __future__ import annotations

import json
from pathlib import Path

from ares_netguard.models.disagreement import generate_disagreement_report
from ares_netguard.models.self_supervised_representation import (
    MODEL_ID,
    REPORT_SCHEMA_VERSION,
    dump_report,
    generate_representation_report,
    load_traffic_sequence_rows,
    representation_evidence_to_score_rows,
)


def test_fixture_generates_representation_report_and_disagreement_rows(tmp_path: Path) -> None:
    fixture = Path("tests/fixtures/self_supervised_representation/synthetic_sequences.jsonl")
    output = tmp_path / "traffic-representation-report.json"

    representation_report = generate_representation_report(load_traffic_sequence_rows(fixture))
    dump_report(representation_report, output)
    persisted = json.loads(output.read_text(encoding="utf-8"))
    persisted_text = output.read_text(encoding="utf-8")

    assert persisted["schema_version"] == REPORT_SCHEMA_VERSION
    assert persisted["rows"]
    assert persisted == json.loads(persisted_text)
    assert not any(
        forbidden in persisted_text.lower()
        for forbidden in (
            "198.51.100.",
            "192.0.2.",
            "example.",
            "http://",
            "https://",
            "password",
            "username",
            "destination_ip",
            "domain",
            "/home/",
            "payload",
        )
    )

    score_rows = representation_evidence_to_score_rows(persisted)
    assert score_rows[0]["schema_version"] == "model_score_row.v0"
    assert all(MODEL_ID in row["scores"] for row in score_rows)

    disagreement_report = generate_disagreement_report(score_rows)
    assert MODEL_ID in disagreement_report["evidence_by_model"]
    assert disagreement_report["model_score_matrix"][0]["scores"][MODEL_ID] >= 0.0
    assert disagreement_report["row_reports"][0]["schema_version"] == "model_score_row.v0"
