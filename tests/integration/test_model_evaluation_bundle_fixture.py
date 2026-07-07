from __future__ import annotations

import json
from pathlib import Path

from ares_netguard.detection_engineering import candidates
from ares_netguard.graph import temporal_security_graph
from ares_netguard.investigation import agentic_layer
from ares_netguard.models import (
    disagreement,
    evaluation_bundle,
    registry_metadata,
    score_row_composer,
    self_supervised_representation,
    time_series_residual,
)
from ares_netguard.native_inference import adapters


def test_fixture_generates_model_evaluation_bundle(tmp_path: Path) -> None:
    disagreement_path = tmp_path / "model-disagreement-report.json"
    residual_path = tmp_path / "time-series-residual-report.json"
    representation_path = tmp_path / "traffic-representation-report.json"
    graph_path = tmp_path / "temporal-security-graph-report.json"
    investigation_path = tmp_path / "agentic-investigation-report.json"
    candidates_path = tmp_path / "detection-candidate-report.json"
    native_scores_path = tmp_path / "native-inference-score-rows.json"
    composed_scores_path = tmp_path / "composed-model-score-rows.json"
    bundle_path = tmp_path / "model-evaluation-bundle.json"
    metadata_path = tmp_path / "model-registry-metadata.json"

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
    score_row_composer.dump_score_rows(
        score_row_composer.compose_score_rows(
            score_row_sources=[
                disagreement.load_score_rows(
                    "tests/fixtures/model_disagreement/synthetic_scores.jsonl"
                ),
                score_row_composer.load_score_rows(native_scores_path),
            ],
            residual_reports=[score_row_composer.load_residual_report(residual_path)],
            representation_reports=[
                score_row_composer.load_representation_report(representation_path)
            ],
            graph_reports=[score_row_composer.load_graph_report(graph_path)],
        ),
        composed_scores_path,
    )
    disagreement.dump_report(
        disagreement.generate_disagreement_report(
            score_row_composer.load_score_rows(composed_scores_path)
        ),
        disagreement_path,
    )
    agentic_layer.dump_report(
        agentic_layer.generate_investigation_report(
            agentic_layer.load_report(disagreement_path),
            evidence_reports=[
                agentic_layer.load_report(residual_path),
                agentic_layer.load_report(representation_path),
                agentic_layer.load_report(graph_path),
            ],
        ),
        investigation_path,
    )
    candidates.dump_report(
        candidates.generate_candidate_report(agentic_layer.load_report(disagreement_path)),
        candidates_path,
    )

    bundle = evaluation_bundle.generate_evaluation_bundle(
        evaluation_bundle.load_bundle_sources(
            [
                disagreement_path,
                residual_path,
                representation_path,
                graph_path,
                investigation_path,
                candidates_path,
                composed_scores_path,
            ]
        )
    )
    evaluation_bundle.dump_bundle(bundle, bundle_path)
    persisted = json.loads(bundle_path.read_text(encoding="utf-8"))
    rendered = json.dumps(persisted, sort_keys=True)
    metadata = registry_metadata.generate_registry_metadata(
        registry_metadata.load_evaluation_bundle(bundle_path)
    )
    registry_metadata.dump_metadata(metadata, metadata_path)
    persisted_metadata = json.loads(metadata_path.read_text(encoding="utf-8"))
    rendered_metadata = json.dumps(persisted_metadata, sort_keys=True)

    assert persisted["schema_version"] == evaluation_bundle.REPORT_SCHEMA_VERSION
    assert persisted == bundle
    assert persisted_metadata["schema_version"] == registry_metadata.REPORT_SCHEMA_VERSION
    assert persisted_metadata == metadata
    assert persisted_metadata["aggregate_summary"]["deployment_allowed"] is False
    assert (
        "stdlib_linear_native" in persisted_metadata["aggregate_summary"]["models_with_score_rows"]
    )
    assert persisted["aggregate_summary"]["schemas_present"] == [
        "agentic_investigation_report.v0",
        "detection_candidate_report.v0",
        "model_disagreement_report.v0",
        "model_score_rows.v0",
        "temporal_security_graph_report.v0",
        "time_series_residual_report.v0",
        "traffic_representation_report.v0",
    ]
    assert persisted["aggregate_summary"]["candidate_count"] == 12
    assert persisted["aggregate_summary"]["hypothesis_count"] == 29
    assert persisted["aggregate_summary"]["score_row_count"] == 30
    assert persisted["source_summaries"][0]["row_count"] == 15
    assert persisted["source_summaries"][5]["candidate_count"] == 12
    assert persisted["source_summaries"][6]["score_row_count"] == 15
    assert "stdlib_linear_native" in persisted["aggregate_summary"]["model_ids"]
    assert persisted["safety_flags"]["local_only"] is True
    assert persisted["safety_flags"]["input_paths_copied"] is False
    assert persisted["safety_flags"]["source_filenames_copied"] is False
    for forbidden in (
        "192.168.",
        "198.51.100.",
        "example.com",
        "http://",
        "https://",
        "/home/",
        str(tmp_path),
        "password",
        "api_key",
        "super_secret_value",
        ".pcap",
        ".parquet",
        ".onnx",
        ".jsonl",
        "model-disagreement-report.json",
        "native-inference-score-rows.json",
        "composed-model-score-rows.json",
        "protocol:tcp",
        "host-alpha",
    ):
        assert forbidden not in rendered
        assert forbidden not in rendered_metadata
