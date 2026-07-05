from __future__ import annotations

import json
from pathlib import Path

import pytest

from ares_netguard.models import evaluation_bundle, registry_metadata
from ares_netguard.models.disagreement import generate_disagreement_report


def _score_row(
    entity_id: str = "host-alpha",
    *,
    model_id: str = "isolation_forest",
) -> dict[str, object]:
    return {
        "schema_version": "model_score_row.v0",
        "entity_id": entity_id,
        "window_start": "2026-01-01T00:00:00Z",
        "scores": {
            model_id: {
                "risk": 0.91,
                "scale": "risk",
                "family": "baseline",
                "evidence": ["synthetic evidence bucket"],
            },
            "pyod_ecod": {
                "risk": 0.88,
                "scale": "risk",
                "family": "pyod",
                "evidence": ["tail probability bucket"],
            },
        },
    }


def _bundle() -> dict[str, object]:
    disagreement_report = generate_disagreement_report([_score_row()])
    native_score_rows = [
        {
            "schema_version": "model_score_row.v0",
            "entity_id": "host-beta",
            "window_start": "2026-01-01T00:05:00Z",
            "scores": {
                "stdlib_linear_native": {
                    "risk": 0.42,
                    "scale": "risk",
                    "family": "native_reference",
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
                }
            },
        }
    ]
    return evaluation_bundle.generate_evaluation_bundle([disagreement_report, native_score_rows])


def test_generates_deterministic_synthetic_only_registry_metadata() -> None:
    metadata = registry_metadata.generate_registry_metadata(_bundle())
    again = registry_metadata.generate_registry_metadata(_bundle())

    assert metadata == again
    assert metadata["schema_version"] == registry_metadata.REPORT_SCHEMA_VERSION
    assert metadata["metadata_scope"] == "local_synthetic_model_registry_metadata"
    assert metadata["source_bundle_schema"] == evaluation_bundle.REPORT_SCHEMA_VERSION
    assert [entry["model_id"] for entry in metadata["entries"]] == [
        "isolation_forest",
        "pyod_ecod",
        "stdlib_linear_native",
    ]
    assert metadata["aggregate_summary"] == {
        "model_count": 3,
        "schemas_present": [
            "model_disagreement_report.v0",
            "model_score_rows.v0",
        ],
        "models_with_score_rows": [
            "isolation_forest",
            "pyod_ecod",
            "stdlib_linear_native",
        ],
        "deployment_allowed": False,
    }
    assert all(
        entry["registry_state"] == "observed_synthetic_only" for entry in metadata["entries"]
    )
    assert all(entry["promotion_state"] == "not_promoted" for entry in metadata["entries"])
    assert all(entry["human_review_required"] is True for entry in metadata["entries"])
    assert all(entry["deployment_allowed"] is False for entry in metadata["entries"])
    assert "host-alpha" not in json.dumps(metadata, sort_keys=True)


def test_load_evaluation_bundle_rejects_unknown_schema(tmp_path: Path) -> None:
    bundle_path = tmp_path / "bundle.json"
    bundle = _bundle()
    bundle["schema_version"] = "model_evaluation_bundle.v1"
    bundle_path.write_text(json.dumps(bundle), encoding="utf-8")

    with pytest.raises(ValueError, match="requires schema_version"):
        registry_metadata.load_evaluation_bundle(bundle_path)


def test_load_evaluation_bundle_rejects_non_strict_json_constants(tmp_path: Path) -> None:
    bundle_path = tmp_path / "bundle.json"
    bundle_path.write_text('{"schema_version":"model_evaluation_bundle.v0","risk":NaN}', "utf-8")

    with pytest.raises(ValueError, match="non-strict JSON constant"):
        registry_metadata.load_evaluation_bundle(bundle_path)


def test_validate_metadata_rejects_tampered_aggregate_summary() -> None:
    metadata = registry_metadata.generate_registry_metadata(_bundle())
    tampered = json.loads(json.dumps(metadata))
    tampered["aggregate_summary"]["model_count"] += 1

    with pytest.raises(ValueError, match="derived from entries"):
        registry_metadata.validate_registry_metadata(tampered)


def test_validate_metadata_rejects_duplicate_model_entries() -> None:
    metadata = registry_metadata.generate_registry_metadata(_bundle())
    tampered = json.loads(json.dumps(metadata))
    tampered["entries"][1] = json.loads(json.dumps(tampered["entries"][0]))
    tampered["aggregate_summary"] = {
        "model_count": len(tampered["entries"]),
        "schemas_present": [
            "model_disagreement_report.v0",
            "model_score_rows.v0",
        ],
        "models_with_score_rows": [
            "isolation_forest",
            "stdlib_linear_native",
        ],
        "deployment_allowed": False,
    }

    with pytest.raises(ValueError, match="one entry per model_id"):
        registry_metadata.validate_registry_metadata(tampered)


def test_validate_metadata_rejects_unsafe_model_id() -> None:
    metadata = registry_metadata.generate_registry_metadata(_bundle())
    tampered = json.loads(json.dumps(metadata))
    tampered["entries"][0]["model_id"] = "InvalidModel"

    with pytest.raises(ValueError, match="sanitized model identifier"):
        registry_metadata.validate_registry_metadata(tampered)


def test_validate_metadata_rejects_source_name_path_leakage() -> None:
    metadata = registry_metadata.generate_registry_metadata(_bundle())
    tampered = json.loads(json.dumps(metadata))
    tampered["entries"][0]["observed_source_names"] = ["/home/sno/model-report.json"]
    tampered["entries"][0]["source_count"] = 1

    with pytest.raises(ValueError, match="unsafe raw identifier"):
        registry_metadata.validate_registry_metadata(tampered)


def test_validate_metadata_rejects_secret_like_fields() -> None:
    metadata = registry_metadata.generate_registry_metadata(_bundle())
    tampered = json.loads(json.dumps(metadata))
    tampered["entries"][0]["api_key"] = "fixture"

    with pytest.raises(ValueError, match="secret-like field"):
        registry_metadata.validate_registry_metadata(tampered)


def test_validate_metadata_rejects_generated_artifact_references() -> None:
    metadata = registry_metadata.generate_registry_metadata(_bundle())
    tampered = json.loads(json.dumps(metadata))
    tampered["entries"][0]["observed_source_names"] = ["model_score_rows_v0_001.onnx"]
    tampered["entries"][0]["source_count"] = 1

    with pytest.raises(ValueError, match="unsafe raw identifier"):
        registry_metadata.validate_registry_metadata(tampered)


def test_dump_metadata_rejects_ordinary_repo_output_path(tmp_path: Path) -> None:
    metadata = registry_metadata.generate_registry_metadata(_bundle())
    repo_root = tmp_path / "repo"
    repo_root.mkdir()

    with pytest.raises(ValueError, match="inside the repository"):
        registry_metadata.dump_metadata(metadata, repo_root / "metadata.json", repo_root=repo_root)

    with pytest.raises(ValueError, match="inside the repository"):
        registry_metadata.dump_metadata(
            metadata, repo_root / "data" / "model-registry-metadata.json", repo_root=repo_root
        )

    output = repo_root / "data" / "registry" / "metadata.json"
    output.parent.mkdir(parents=True)
    registry_metadata.dump_metadata(metadata, output, repo_root=repo_root)
    assert json.loads(output.read_text(encoding="utf-8")) == metadata


def test_cli_rejects_repo_output_when_invoked_outside_repo(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    bundle_path = tmp_path / "model-evaluation-bundle.json"
    evaluation_bundle.dump_bundle(_bundle(), bundle_path)
    repo_root = Path(__file__).resolve().parents[2]
    monkeypatch.chdir(tmp_path)

    with pytest.raises(ValueError, match="inside the repository"):
        registry_metadata.main(
            [
                str(repo_root / "docs" / "generated-model-registry-metadata.json"),
                str(bundle_path),
            ]
        )
