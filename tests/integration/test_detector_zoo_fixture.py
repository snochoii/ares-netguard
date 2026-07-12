from __future__ import annotations

import json
from pathlib import Path

from ares_netguard.features import evidence_windows
from ares_netguard.models import (
    detector_zoo,
    disagreement,
    evaluation_bundle,
    registry_metadata,
    score_row_composer,
)
from ares_netguard.storage import evidence_index


def test_detector_zoo_flows_through_evaluation_registry_and_evidence_index(
    tmp_path: Path,
) -> None:
    feature_path = tmp_path / "detector-feature-windows.json"
    detector_scores_path = tmp_path / "detector-score-rows.json"
    composed_path = tmp_path / "composed-score-rows.json"
    disagreement_path = tmp_path / "disagreement-report.json"
    bundle_path = tmp_path / "evaluation-bundle.json"
    metadata_path = tmp_path / "registry-metadata.json"
    index_path = tmp_path / "evidence-index.json"

    feature_report = evidence_windows.generate_feature_window_report(
        evidence_windows.load_synthetic_telemetry_events(
            "tests/fixtures/detector_zoo/synthetic_events.jsonl"
        ),
        window_sizes_minutes=(5,),
    )
    evidence_windows.dump_feature_window_report(feature_report, feature_path)
    detector_zoo.dump_score_rows(
        detector_zoo.generate_detector_score_rows(
            detector_zoo.load_feature_window_report(feature_path)
        ),
        detector_scores_path,
    )

    composed_rows = score_row_composer.compose_score_rows(
        score_row_sources=[
            score_row_composer.load_score_rows(
                "tests/fixtures/model_disagreement/synthetic_scores.jsonl"
            ),
            score_row_composer.load_score_rows(detector_scores_path),
        ]
    )
    score_row_composer.dump_score_rows(composed_rows, composed_path)
    disagreement.dump_report(
        disagreement.generate_disagreement_report(composed_rows),
        disagreement_path,
    )

    bundle = evaluation_bundle.generate_evaluation_bundle(
        evaluation_bundle.load_bundle_sources([disagreement_path, composed_path])
    )
    evaluation_bundle.dump_bundle(bundle, bundle_path)
    metadata = registry_metadata.generate_registry_metadata(
        registry_metadata.load_evaluation_bundle(bundle_path)
    )
    registry_metadata.dump_metadata(metadata, metadata_path)
    index = evidence_index.generate_evidence_index(
        evidence_index.load_evidence_sources(
            [feature_path, disagreement_path, composed_path, metadata_path]
        )
    )
    evidence_index.dump_evidence_index(index, index_path)

    persisted_scores = json.loads(detector_scores_path.read_text(encoding="utf-8"))
    persisted_bundle = json.loads(bundle_path.read_text(encoding="utf-8"))
    persisted_metadata = json.loads(metadata_path.read_text(encoding="utf-8"))
    persisted_index = json.loads(index_path.read_text(encoding="utf-8"))

    assert len(persisted_scores) == 48
    assert len(composed_rows) == 51
    assert len(json.loads(disagreement_path.read_text(encoding="utf-8"))["row_reports"]) == 51
    assert set(detector_zoo.MODEL_IDS) <= set(persisted_bundle["aggregate_summary"]["model_ids"])
    assert set(detector_zoo.MODEL_IDS) <= set(
        persisted_metadata["aggregate_summary"]["models_with_score_rows"]
    )
    assert all(
        entry["registry_state"] == "observed_synthetic_only"
        and entry["deployment_allowed"] is False
        for entry in persisted_metadata["entries"]
        if entry["model_id"] in detector_zoo.MODEL_IDS
    )
    assert set(detector_zoo.MODEL_IDS) <= set(persisted_index["aggregate_summary"]["model_ids"])
    assert set(detector_zoo.FEATURE_COLUMNS) <= set(
        persisted_index["aggregate_summary"]["feature_names"]
    )
    assert any(row["entity_id"] == "host-zoo" for row in persisted_index["entity_window_index"])
    assert persisted_index["safety_flags"]["raw_evidence_payload_copied"] is False
    assert persisted_index["safety_flags"]["input_paths_copied"] is False
    assert "detector-feature-windows.json" not in json.dumps(persisted_index, sort_keys=True)
