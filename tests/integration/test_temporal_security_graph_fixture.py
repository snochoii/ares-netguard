from __future__ import annotations

import json
from pathlib import Path

from ares_netguard.graph.temporal_security_graph import (
    MODEL_ID,
    REPORT_SCHEMA_VERSION,
    dump_report,
    generate_temporal_security_graph_report,
    load_temporal_graph_edge_rows,
    temporal_graph_evidence_to_score_rows,
)
from ares_netguard.models.disagreement import generate_disagreement_report


def test_fixture_generates_graph_report_and_disagreement_rows(tmp_path: Path) -> None:
    fixture = Path("tests/fixtures/temporal_security_graph/synthetic_edges.jsonl")
    output = tmp_path / "temporal-security-graph-report.json"

    graph_report = generate_temporal_security_graph_report(load_temporal_graph_edge_rows(fixture))
    dump_report(graph_report, output)
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
            "alice@example",
            "/home/",
            "payload",
            "api_key",
        )
    )

    score_rows = temporal_graph_evidence_to_score_rows(persisted)
    assert score_rows[0]["schema_version"] == "model_score_row.v0"
    assert all(MODEL_ID in row["scores"] for row in score_rows)
    assert max(row["scores"][MODEL_ID]["risk"] for row in score_rows) == 1.0

    disagreement_report = generate_disagreement_report(score_rows)
    assert MODEL_ID in disagreement_report["evidence_by_model"]
    assert disagreement_report["model_score_matrix"][0]["scores"][MODEL_ID] >= 0.0
    assert disagreement_report["row_reports"][0]["schema_version"] == "model_score_row.v0"
