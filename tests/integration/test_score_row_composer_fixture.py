from __future__ import annotations

import json
from pathlib import Path

from ares_netguard.graph import temporal_security_graph
from ares_netguard.models import (
    disagreement,
    score_row_composer,
    self_supervised_representation,
    time_series_residual,
)
from ares_netguard.native_inference import adapters


def test_fixture_composes_current_score_producers_for_disagreement(tmp_path: Path) -> None:
    residual_path = tmp_path / "time-series-residual-report.json"
    representation_path = tmp_path / "traffic-representation-report.json"
    graph_path = tmp_path / "temporal-security-graph-report.json"
    native_scores_path = tmp_path / "native-inference-score-rows.json"
    composed_path = tmp_path / "composed-model-score-rows.json"
    disagreement_path = tmp_path / "model-disagreement-report.json"

    time_series_residual.dump_report(
        time_series_residual.generate_residual_report(
            time_series_residual.load_time_window_rows(
                "tests/fixtures/time_series_residual/synthetic_windows.jsonl"
            )
        ),
        residual_path,
    )
    self_supervised_representation.dump_report(
        self_supervised_representation.generate_representation_report(
            self_supervised_representation.load_traffic_sequence_rows(
                "tests/fixtures/self_supervised_representation/synthetic_sequences.jsonl"
            )
        ),
        representation_path,
    )
    temporal_security_graph.dump_report(
        temporal_security_graph.generate_temporal_security_graph_report(
            temporal_security_graph.load_temporal_graph_edge_rows(
                "tests/fixtures/temporal_security_graph/synthetic_edges.jsonl"
            )
        ),
        graph_path,
    )
    adapters.dump_score_rows(
        adapters.score_feature_rows(
            adapters.load_manifest("tests/fixtures/native_inference/manifest.json"),
            adapters.load_feature_rows("tests/fixtures/native_inference/feature_rows.jsonl"),
        ),
        native_scores_path,
    )

    assert (
        score_row_composer.main(
            [
                str(composed_path),
                "--score-rows",
                "tests/fixtures/model_disagreement/synthetic_scores.jsonl",
                "--score-rows",
                str(native_scores_path),
                "--residual-report",
                str(residual_path),
                "--representation-report",
                str(representation_path),
                "--graph-report",
                str(graph_path),
            ]
        )
        == 0
    )
    composed_rows = json.loads(composed_path.read_text(encoding="utf-8"))
    disagreement_report = disagreement.generate_disagreement_report(composed_rows)
    disagreement.dump_report(disagreement_report, disagreement_path)
    persisted_disagreement = json.loads(disagreement_path.read_text(encoding="utf-8"))

    assert isinstance(composed_rows, list)
    assert len(composed_rows) == 15
    assert composed_rows == sorted(
        composed_rows,
        key=lambda row: (row["entity_id"], row["window_start"]),
    )
    assert persisted_disagreement["row_reports"] == disagreement_report["row_reports"]
    assert len(persisted_disagreement["row_reports"]) == 15
    assert persisted_disagreement["model_agreement_score"] == 0.875287
    assert persisted_disagreement["model_disagreement_score"] == 0.124713
    assert persisted_disagreement["consensus_risk"] == 0.459949
    assert persisted_disagreement["outlier_model"] == "stdlib_linear_native"
    assert sorted(persisted_disagreement["evidence_by_model"]) == [
        "graph_novelty",
        "isolation_forest",
        "pyod_copod",
        "pyod_ecod",
        "river_hst",
        "self_supervised_representation",
        "stdlib_linear_native",
        "suricata_alert",
        "time_series_residual",
    ]
