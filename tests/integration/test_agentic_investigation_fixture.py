from __future__ import annotations

import json
from pathlib import Path

from ares_netguard.graph.temporal_security_graph import (
    generate_temporal_security_graph_report,
    load_temporal_graph_edge_rows,
)
from ares_netguard.investigation.agentic_layer import (
    REPORT_SCHEMA_VERSION,
    generate_investigation_report,
)
from ares_netguard.investigation.agentic_layer import (
    dump_report as dump_investigation_report,
)
from ares_netguard.investigation.agentic_layer import (
    load_report as load_investigation_input,
)
from ares_netguard.models.disagreement import (
    dump_report as dump_disagreement_report,
)
from ares_netguard.models.disagreement import (
    generate_disagreement_report,
    load_score_rows,
)
from ares_netguard.models.self_supervised_representation import (
    generate_representation_report,
    load_traffic_sequence_rows,
)
from ares_netguard.models.time_series_residual import (
    generate_residual_report,
    load_time_window_rows,
)


def test_fixture_generates_agentic_investigation_report(tmp_path: Path) -> None:
    disagreement_path = tmp_path / "model-disagreement-report.json"
    residual_path = tmp_path / "time-series-residual-report.json"
    representation_path = tmp_path / "traffic-representation-report.json"
    graph_path = tmp_path / "temporal-security-graph-report.json"
    output_path = tmp_path / "agentic-investigation-report.json"

    dump_disagreement_report(
        generate_disagreement_report(
            load_score_rows("tests/fixtures/model_disagreement/synthetic_scores.jsonl")
        ),
        disagreement_path,
    )
    residual_path.write_text(
        json.dumps(
            generate_residual_report(
                load_time_window_rows("tests/fixtures/time_series_residual/synthetic_windows.jsonl")
            ),
            allow_nan=False,
            indent=2,
            sort_keys=True,
        )
        + "\n",
        encoding="utf-8",
    )
    representation_path.write_text(
        json.dumps(
            generate_representation_report(
                load_traffic_sequence_rows(
                    "tests/fixtures/self_supervised_representation/synthetic_sequences.jsonl"
                )
            ),
            allow_nan=False,
            indent=2,
            sort_keys=True,
        )
        + "\n",
        encoding="utf-8",
    )
    graph_path.write_text(
        json.dumps(
            generate_temporal_security_graph_report(
                load_temporal_graph_edge_rows(
                    "tests/fixtures/temporal_security_graph/synthetic_edges.jsonl"
                )
            ),
            allow_nan=False,
            indent=2,
            sort_keys=True,
        )
        + "\n",
        encoding="utf-8",
    )

    report = generate_investigation_report(
        load_investigation_input(disagreement_path),
        evidence_reports=[
            load_investigation_input(residual_path),
            load_investigation_input(representation_path),
            load_investigation_input(graph_path),
        ],
    )
    dump_investigation_report(report, output_path)
    persisted = json.loads(output_path.read_text(encoding="utf-8"))
    rendered = json.dumps(persisted, sort_keys=True)

    assert persisted["schema_version"] == REPORT_SCHEMA_VERSION
    assert len(persisted["rows"]) >= 4
    assert any("high consensus" in row["claim"] for row in persisted["rows"])
    assert any("outlier model" in row["claim"] for row in persisted["rows"])
    assert any("sparse or missing" in row["claim"] for row in persisted["rows"])
    assert any("matching local evidence reports" in row["claim"] for row in persisted["rows"])
    assert "traffic_representation_report.v0" in rendered
    assert "protocol:tcp" not in rendered
    for forbidden in ("192.168.", "example.com", "http://", "/home/", "password", "secret"):
        assert forbidden not in rendered
