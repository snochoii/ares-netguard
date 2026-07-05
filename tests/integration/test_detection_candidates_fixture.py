from __future__ import annotations

import json
from pathlib import Path

from ares_netguard.detection_engineering import candidates
from ares_netguard.models import disagreement


def test_fixture_generates_detection_candidate_report(tmp_path: Path) -> None:
    fixture = Path("tests/fixtures/model_disagreement/synthetic_scores.jsonl")
    disagreement_output = tmp_path / "model-disagreement-report.json"
    candidate_output = tmp_path / "detection-candidate-report.json"

    disagreement_report = disagreement.generate_disagreement_report(
        disagreement.load_score_rows(fixture)
    )
    disagreement.dump_report(disagreement_report, disagreement_output)

    candidate_report = candidates.generate_candidate_report(
        json.loads(disagreement_output.read_text(encoding="utf-8"))
    )
    candidates.dump_report(candidate_report, candidate_output)
    persisted = json.loads(candidate_output.read_text(encoding="utf-8"))

    assert persisted["schema_version"] == candidates.REPORT_SCHEMA_VERSION
    assert persisted["source_report_schema"] == "model_disagreement_report.v0"
    assert persisted["validation_summary"]["candidates_generated"] == 8
    assert {row["candidate_language"] for row in persisted["rows"]} == set(
        candidates.CANDIDATE_LANGUAGES
    )
    assert {row["candidate_kind"] for row in persisted["rows"]} == {
        "high_consensus_risk",
        "high_model_disagreement",
    }
    rendered = json.dumps(persisted, sort_keys=True)
    assert "192.168." not in rendered
    assert "example.com" not in rendered
    assert "external destination diversity spike" not in rendered
    assert persisted == json.loads(candidate_output.read_text(encoding="utf-8"))
