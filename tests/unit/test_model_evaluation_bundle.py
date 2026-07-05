from __future__ import annotations

import json
from pathlib import Path

import pytest

from ares_netguard.detection_engineering import candidates
from ares_netguard.investigation import agentic_layer
from ares_netguard.models import evaluation_bundle
from ares_netguard.models.disagreement import generate_disagreement_report


def _score_row(
    entity_id: str = "host-alpha",
    *,
    evidence: list[object] | None = None,
) -> dict[str, object]:
    return {
        "schema_version": "model_score_row.v0",
        "entity_id": entity_id,
        "window_start": "2026-01-01T00:00:00Z",
        "scores": {
            "isolation_forest": {
                "risk": 0.91,
                "scale": "risk",
                "family": "baseline",
                "evidence": evidence or ["synthetic evidence bucket"],
            },
            "pyod_ecod": {
                "risk": 0.88,
                "scale": "risk",
                "family": "pyod",
                "evidence": ["tail probability bucket"],
            },
        },
    }


def _bundle_sources() -> list[object]:
    disagreement_report = generate_disagreement_report([_score_row()])
    investigation_report = agentic_layer.generate_investigation_report(disagreement_report)
    candidate_report = candidates.generate_candidate_report(disagreement_report)
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
    return [disagreement_report, investigation_report, candidate_report, native_score_rows]


def test_generates_deterministic_aggregate_only_bundle() -> None:
    bundle = evaluation_bundle.generate_evaluation_bundle(_bundle_sources())
    again = evaluation_bundle.generate_evaluation_bundle(_bundle_sources())

    assert bundle == again
    assert bundle["schema_version"] == evaluation_bundle.REPORT_SCHEMA_VERSION
    assert bundle["aggregate_summary"]["schemas_present"] == [
        "agentic_investigation_report.v0",
        "detection_candidate_report.v0",
        "model_disagreement_report.v0",
        "model_score_rows.v0",
    ]
    assert bundle["aggregate_summary"]["score_row_count"] == 2
    assert bundle["aggregate_summary"]["hypothesis_count"] >= 1
    assert bundle["aggregate_summary"]["candidate_count"] == 4
    assert bundle["aggregate_summary"]["candidate_languages"] == sorted(
        candidates.CANDIDATE_LANGUAGES
    )
    assert bundle["aggregate_summary"]["model_ids"] == [
        "isolation_forest",
        "model_disagreement",
        "pyod_ecod",
        "stdlib_linear_native",
    ]
    assert all("source_name" in summary for summary in bundle["source_summaries"])
    assert "host-alpha" not in json.dumps(bundle, sort_keys=True)


def test_load_source_does_not_copy_path_or_filename_into_source_name(tmp_path: Path) -> None:
    source_path = tmp_path / "192.168.1.9-secret-model-disagreement-report.json"
    source_path.write_text(
        json.dumps(generate_disagreement_report([_score_row()]), allow_nan=False),
        encoding="utf-8",
    )

    bundle = evaluation_bundle.generate_evaluation_bundle(
        evaluation_bundle.load_bundle_sources([source_path])
    )
    rendered = json.dumps(bundle, sort_keys=True)

    assert bundle["source_summaries"][0]["source_name"] == "model_disagreement_report_v0_001"
    assert "192.168.1.9" not in rendered
    assert "secret-model-disagreement-report" not in rendered
    assert str(tmp_path) not in rendered


def test_unknown_report_schema_is_rejected() -> None:
    with pytest.raises(ValueError, match="unknown report schema_version"):
        evaluation_bundle.generate_evaluation_bundle(
            [{"schema_version": "surprise.v0", "rows": []}]
        )


def test_non_strict_json_constants_are_rejected(tmp_path: Path) -> None:
    source_path = tmp_path / "bad.json"
    source_path.write_text('{"schema_version":"model_disagreement_report.v0","risk":NaN}', "utf-8")

    with pytest.raises(ValueError, match="non-strict JSON constant"):
        evaluation_bundle.load_bundle_source(source_path)


def test_raw_identifiers_are_rejected() -> None:
    with pytest.raises(ValueError, match="unsafe raw identifier"):
        evaluation_bundle.generate_evaluation_bundle([[_score_row("192.168.1.20")]])


def test_secret_like_fields_are_rejected() -> None:
    report = _score_row()
    report["api_key"] = "fixture"

    with pytest.raises(ValueError, match="secret-like field"):
        evaluation_bundle.generate_evaluation_bundle([[report]])  # type: ignore[list-item]


def test_path_like_private_values_are_rejected() -> None:
    with pytest.raises(ValueError, match="unsafe raw identifier"):
        evaluation_bundle.generate_evaluation_bundle([[_score_row(evidence=["/home/sno/private"])]])


def test_generated_artifact_references_are_rejected() -> None:
    with pytest.raises(ValueError, match="unsafe raw identifier"):
        evaluation_bundle.generate_evaluation_bundle([[_score_row(evidence=["model.onnx"])]])


def test_dump_bundle_rejects_ordinary_repo_output_path(tmp_path: Path) -> None:
    bundle = evaluation_bundle.generate_evaluation_bundle([_bundle_sources()[0]])
    repo_root = tmp_path / "repo"
    repo_root.mkdir()

    with pytest.raises(ValueError, match="inside the repository"):
        evaluation_bundle.dump_bundle(bundle, repo_root / "report.json", repo_root=repo_root)

    output = repo_root / "data" / "reports" / "bundle.json"
    output.parent.mkdir(parents=True)
    evaluation_bundle.dump_bundle(bundle, output, repo_root=repo_root)
    assert json.loads(output.read_text(encoding="utf-8")) == bundle
