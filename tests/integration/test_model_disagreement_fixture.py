from __future__ import annotations

import json
from pathlib import Path

from ares_netguard.models.disagreement import (
    REPORT_SCHEMA_VERSION,
    dump_report,
    generate_disagreement_report,
    load_score_rows,
)


def test_fixture_generates_required_disagreement_report(tmp_path: Path) -> None:
    fixture = Path("tests/fixtures/model_disagreement/synthetic_scores.jsonl")
    output = tmp_path / "report.json"

    report = generate_disagreement_report(load_score_rows(fixture))
    dump_report(report, output)
    persisted = json.loads(output.read_text(encoding="utf-8"))

    assert persisted["schema_version"] == REPORT_SCHEMA_VERSION
    assert persisted["model_score_matrix"]
    assert "model_agreement_score" in persisted
    assert "model_disagreement_score" in persisted
    assert "consensus_risk" in persisted
    assert "outlier_model" in persisted
    assert persisted["top_supporting_models"]
    assert persisted["top_dissenting_models"]
    assert "evidence_by_model" in persisted
    assert persisted == json.loads(output.read_text(encoding="utf-8"))
