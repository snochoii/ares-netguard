from __future__ import annotations

import json
from pathlib import Path

from ares_netguard.detection_engineering import candidates
from ares_netguard.features import evidence_windows
from ares_netguard.graph import temporal_security_graph
from ares_netguard.investigation import agentic_layer
from ares_netguard.models import (
    disagreement,
    evaluation_bundle,
    registry_metadata,
    self_supervised_representation,
    time_series_residual,
)
from ares_netguard.native_inference import adapters
from ares_netguard.storage import evidence_index


def test_fixture_generates_evidence_index(tmp_path: Path) -> None:
    telemetry_path = tmp_path / "telemetry-feature-windows.json"
    disagreement_path = tmp_path / "model-disagreement-report.json"
    residual_path = tmp_path / "time-series-residual-report.json"
    representation_path = tmp_path / "traffic-representation-report.json"
    graph_path = tmp_path / "temporal-security-graph-report.json"
    investigation_path = tmp_path / "agentic-investigation-report.json"
    candidates_path = tmp_path / "detection-candidate-report.json"
    native_scores_path = tmp_path / "native-inference-score-rows.json"
    bundle_path = tmp_path / "model-evaluation-bundle.json"
    metadata_path = tmp_path / "model-registry-metadata.json"
    index_path = tmp_path / "evidence-index.json"

    evidence_windows.dump_feature_window_report(
        evidence_windows.generate_feature_window_report(
            evidence_windows.load_synthetic_telemetry_events(
                "tests/fixtures/telemetry_foundation/synthetic_events.jsonl"
            )
        ),
        telemetry_path,
    )
    disagreement.dump_report(
        disagreement.generate_disagreement_report(
            disagreement.load_score_rows("tests/fixtures/model_disagreement/synthetic_scores.jsonl")
        ),
        disagreement_path,
    )
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
    adapters.dump_score_rows(
        adapters.score_feature_rows(
            adapters.load_manifest("tests/fixtures/native_inference/manifest.json"),
            adapters.load_feature_rows("tests/fixtures/native_inference/feature_rows.jsonl"),
        ),
        native_scores_path,
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
                native_scores_path,
            ]
        )
    )
    evaluation_bundle.dump_bundle(bundle, bundle_path)
    metadata = registry_metadata.generate_registry_metadata(
        registry_metadata.load_evaluation_bundle(bundle_path)
    )
    registry_metadata.dump_metadata(metadata, metadata_path)

    index = evidence_index.generate_evidence_index(
        evidence_index.load_evidence_sources(
            [
                telemetry_path,
                disagreement_path,
                residual_path,
                representation_path,
                graph_path,
                investigation_path,
                candidates_path,
                native_scores_path,
                metadata_path,
            ]
        )
    )
    evidence_index.dump_evidence_index(index, index_path)
    persisted = json.loads(index_path.read_text(encoding="utf-8"))
    rendered = json.dumps(persisted, sort_keys=True)

    assert persisted == index
    assert persisted["schema_version"] == evidence_index.EVIDENCE_INDEX_SCHEMA_VERSION
    assert persisted["aggregate_summary"]["schemas_present"] == [
        "agentic_investigation_report.v0",
        "detection_candidate_report.v0",
        "model_disagreement_report.v0",
        "model_registry_metadata.v0",
        "model_score_rows.v0",
        "telemetry_feature_window_report.v0",
        "temporal_security_graph_report.v0",
        "time_series_residual_report.v0",
        "traffic_representation_report.v0",
    ]
    assert persisted["aggregate_summary"]["source_count"] == 9
    assert persisted["aggregate_summary"]["entity_window_count"] >= 3
    assert "dns_failure_ratio" in persisted["aggregate_summary"]["feature_names"]
    assert "stdlib_linear_native" in persisted["aggregate_summary"]["model_ids"]
    assert persisted["safety_flags"]["input_paths_copied"] is False
    assert persisted["safety_flags"]["raw_evidence_payload_copied"] is False
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
        "protocol:tcp",
        "DRAFT_DO_NOT_DEPLOY",
        "Review local evidence refs",
    ):
        assert forbidden not in rendered
