from __future__ import annotations

import json
from pathlib import Path

import pytest

from ares_netguard.detection_engineering import candidates
from ares_netguard.features import evidence_windows
from ares_netguard.investigation import agentic_layer
from ares_netguard.models import registry_metadata
from ares_netguard.models.disagreement import generate_disagreement_report
from ares_netguard.storage import evidence_index


def _score_row(entity_id: str = "host-alpha") -> dict[str, object]:
    return {
        "schema_version": "model_score_row.v0",
        "entity_id": entity_id,
        "window_start": "2026-01-01T00:00:00Z",
        "scores": {
            "isolation_forest": {
                "risk": 0.91,
                "scale": "risk",
                "family": "baseline",
                "evidence": ["external destination diversity spike"],
            },
            "pyod_ecod": {
                "risk": 0.88,
                "scale": "risk",
                "family": "pyod",
                "evidence": [
                    {
                        "feature_columns": ["dns_failure_ratio"],
                        "feature_contributions": [
                            {
                                "feature_name": "dns_failure_ratio",
                                "feature_value": 0.2,
                                "weight": 1.0,
                                "contribution": 0.2,
                            }
                        ],
                    }
                ],
            },
        },
    }


def _feature_report() -> dict[str, object]:
    return evidence_windows.generate_feature_window_report(
        [
            {
                "schema_version": "synthetic_telemetry_event.v0",
                "source_kind": "zeek_dns",
                "entity_id": "host-alpha",
                "timestamp": "2026-01-01T00:00:00Z",
                "event_count": 1,
                "connection_count": 1,
                "dns_query_count": 1,
                "dns_failure_count": 1,
                "alert_severity": 0,
                "bytes_in": 10.0,
                "bytes_out": 20.0,
                "duration_ms": 30.0,
                "destination_asset_id": "asset-beta",
                "service_name": "dns",
                "tls_unknown": False,
                "runtime_event_count": 0,
            }
        ],
        window_sizes_minutes=(1,),
    )


def _registry_metadata() -> dict[str, object]:
    return {
        "schema_version": registry_metadata.REPORT_SCHEMA_VERSION,
        "metadata_scope": registry_metadata.METADATA_SCOPE,
        "source_bundle_schema": "model_evaluation_bundle.v0",
        "entries": [
            {
                "model_id": "isolation_forest",
                "registry_state": registry_metadata.REGISTRY_STATE,
                "promotion_state": registry_metadata.PROMOTION_STATE,
                "observed_source_schemas": ["model_disagreement_report.v0"],
                "observed_source_names": ["model_disagreement_report_v0_001"],
                "source_count": 1,
                "has_score_rows": True,
                "human_review_required": True,
                "deployment_allowed": False,
            }
        ],
        "aggregate_summary": {
            "model_count": 1,
            "schemas_present": ["model_disagreement_report.v0"],
            "models_with_score_rows": ["isolation_forest"],
            "deployment_allowed": False,
        },
        "safety_flags": {
            "local_only": True,
            "strict_json_loaded": True,
            "derived_from_evaluation_bundle_only": True,
            "input_paths_copied": False,
            "source_filenames_copied": False,
            "raw_identifiers_copied": False,
            "generated_artifact_references_copied": False,
            "secrets_detected": False,
            "report_payload_copied": False,
            "live_capture_used": False,
            "external_services_used": False,
            "deployment_allowed": False,
        },
        "non_claims": [
            "not_persistent_model_registry",
            "not_model_promotion_gate",
            "not_deployment_approval",
            "not_live_capture",
            "not_external_enrichment",
            "not_rule_deployment",
            "not_native_runtime_execution",
        ],
    }


def test_generates_deterministic_pointer_only_index() -> None:
    disagreement = generate_disagreement_report([_score_row()])
    investigation = agentic_layer.generate_investigation_report(disagreement)
    detection = candidates.generate_candidate_report(disagreement)

    index = evidence_index.generate_evidence_index(
        [
            _feature_report(),
            disagreement,
            investigation,
            detection,
            [_score_row()],
            _registry_metadata(),
        ]
    )
    again = evidence_index.generate_evidence_index(
        [
            _feature_report(),
            disagreement,
            investigation,
            detection,
            [_score_row()],
            _registry_metadata(),
        ]
    )
    rendered = json.dumps(index, sort_keys=True)

    assert index == again
    assert index["schema_version"] == evidence_index.EVIDENCE_INDEX_SCHEMA_VERSION
    assert index["aggregate_summary"]["schemas_present"] == [
        "agentic_investigation_report.v0",
        "detection_candidate_report.v0",
        "model_disagreement_report.v0",
        "model_registry_metadata.v0",
        "model_score_rows.v0",
        "telemetry_feature_window_report.v0",
    ]
    assert index["entity_window_index"][0]["entity_id"] == "host-alpha"
    assert "dns_failure_ratio" in index["entity_window_index"][0]["feature_names"]
    assert "isolation_forest" in index["entity_window_index"][0]["model_ids"]
    assert index["safety_flags"]["pointer_only"] is True
    assert "external destination diversity spike" not in rendered
    assert "tail probability bucket" not in rendered
    assert "DRAFT_DO_NOT_DEPLOY" not in rendered


def test_load_source_does_not_copy_path_or_filename_into_source_name(tmp_path: Path) -> None:
    source_path = tmp_path / "192.168.1.9-secret-model-disagreement-report.json"
    source_path.write_text(
        json.dumps(generate_disagreement_report([_score_row()]), allow_nan=False),
        encoding="utf-8",
    )

    index = evidence_index.generate_evidence_index(
        evidence_index.load_evidence_sources([source_path])
    )
    rendered = json.dumps(index, sort_keys=True)

    assert index["source_summaries"][0]["source_name"] == "model_disagreement_report_v0_001"
    assert "192.168.1.9" not in rendered
    assert "secret-model-disagreement-report" not in rendered
    assert str(tmp_path) not in rendered


def test_unknown_report_schema_is_rejected() -> None:
    with pytest.raises(ValueError, match="unknown report schema_version"):
        evidence_index.generate_evidence_index([{"schema_version": "surprise.v0", "rows": []}])


def test_direct_sources_are_validated_before_indexing() -> None:
    row = _score_row()
    row["scores"]["pyod_ecod"]["evidence"] = ["model.onnx"]  # type: ignore[index]

    with pytest.raises(ValueError, match="unsafe raw identifier"):
        evidence_index.generate_evidence_index([[row]])  # type: ignore[list-item]


def test_non_strict_json_constants_are_rejected(tmp_path: Path) -> None:
    source_path = tmp_path / "bad.json"
    source_path.write_text('{"schema_version":"model_disagreement_report.v0","risk":NaN}', "utf-8")

    with pytest.raises(ValueError, match="non-strict JSON constant"):
        evidence_index.load_evidence_source(source_path)


def test_validate_index_rejects_tampered_aggregate() -> None:
    index = evidence_index.generate_evidence_index([[_score_row()]])
    tampered = json.loads(json.dumps(index))
    tampered["aggregate_summary"]["source_ref_count"] += 1

    with pytest.raises(ValueError, match="aggregate_summary must be derived"):
        evidence_index.validate_evidence_index(tampered)


def test_dump_index_rejects_ordinary_repo_output_path(tmp_path: Path) -> None:
    index = evidence_index.generate_evidence_index([[_score_row()]])
    repo_root = tmp_path / "repo"
    repo_root.mkdir()

    with pytest.raises(ValueError, match="inside the repository"):
        evidence_index.dump_evidence_index(
            index, repo_root / "evidence-index.json", repo_root=repo_root
        )

    output = repo_root / "data" / "reports" / "evidence-index.json"
    output.parent.mkdir(parents=True)
    evidence_index.dump_evidence_index(index, output, repo_root=repo_root)
    assert json.loads(output.read_text(encoding="utf-8")) == index
