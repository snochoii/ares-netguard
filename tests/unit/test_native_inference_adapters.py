from __future__ import annotations

from pathlib import Path

import pytest

from ares_netguard.native_inference.adapters import (
    FEATURE_ROW_SCHEMA_VERSION,
    MANIFEST_SCHEMA_VERSION,
    dump_score_rows,
    load_manifest,
    score_feature_rows,
)


def _manifest(
    *,
    feature_columns: list[str] | None = None,
    weights: list[float] | None = None,
) -> dict[str, object]:
    columns = feature_columns or ["dns_failure_ratio", "external_connection_count"]
    return {
        "schema_version": MANIFEST_SCHEMA_VERSION,
        "model_id": "stdlib_linear_native",
        "model_family": "native_reference",
        "feature_schema_version": FEATURE_ROW_SCHEMA_VERSION,
        "feature_columns": columns,
        "training_data_summary": {"scope": "synthetic_fixture", "rows": 1},
        "evaluation_summary": {"scope": "schema_contract_only", "rows": 1},
        "calibration_summary": {"method": "logistic_reference"},
        "export_format": "stdlib_linear_score.v0",
        "inference_runtime": "stdlib_reference",
        "privacy_safety_notes": ["synthetic feature vectors only"],
        "adapter": {
            "kind": "linear_score.v0",
            "weights": weights or [2.0, 0.05],
            "bias": -1.0,
            "normalization": "logistic",
        },
    }


def _row(
    *,
    features: dict[str, object] | None = None,
    entity_id: str = "host-alpha",
) -> dict[str, object]:
    return {
        "schema_version": FEATURE_ROW_SCHEMA_VERSION,
        "entity_id": entity_id,
        "window_start": "2026-01-01T00:00:00Z",
        "features": features
        or {
            "dns_failure_ratio": 0.5,
            "external_connection_count": 20,
        },
    }


def test_valid_manifest_and_feature_rows_emit_model_score_rows() -> None:
    rows = score_feature_rows(_manifest(), [_row()])

    assert rows == [
        {
            "schema_version": "model_score_row.v0",
            "entity_id": "host-alpha",
            "window_start": "2026-01-01T00:00:00Z",
            "scores": {
                "stdlib_linear_native": {
                    "risk": 0.731059,
                    "scale": "risk",
                    "family": "native_reference",
                    "evidence": [
                        {
                            "adapter_kind": "linear_score.v0",
                            "feature_schema_version": FEATURE_ROW_SCHEMA_VERSION,
                            "feature_columns": [
                                "dns_failure_ratio",
                                "external_connection_count",
                            ],
                            "linear_score": 1.0,
                            "normalization": "logistic",
                            "feature_contributions": [
                                {
                                    "feature_name": "dns_failure_ratio",
                                    "feature_value": 0.5,
                                    "weight": 2.0,
                                    "contribution": 1.0,
                                },
                                {
                                    "feature_name": "external_connection_count",
                                    "feature_value": 20.0,
                                    "weight": 0.05,
                                    "contribution": 1.0,
                                },
                            ],
                        }
                    ],
                }
            },
        }
    ]


def test_feature_column_order_controls_weight_alignment() -> None:
    features = {"a_score": 1.0, "b_score": 2.0}
    first = score_feature_rows(
        _manifest(feature_columns=["a_score", "b_score"], weights=[1.0, 2.0]),
        [_row(features=features)],
    )
    second = score_feature_rows(
        _manifest(feature_columns=["b_score", "a_score"], weights=[1.0, 2.0]),
        [_row(features=features)],
    )

    assert first[0]["scores"]["stdlib_linear_native"]["risk"] == 0.982014
    assert second[0]["scores"]["stdlib_linear_native"]["risk"] == 0.952574


def test_strict_manifest_fields_are_required() -> None:
    manifest = _manifest()
    manifest["artifact_path"] = "model"

    with pytest.raises(ValueError, match="native inference manifest fields invalid"):
        score_feature_rows(manifest, [_row()])


def test_unknown_schema_versions_are_rejected() -> None:
    manifest = _manifest()
    manifest["schema_version"] = "native_inference_manifest.v1"

    with pytest.raises(ValueError, match="unknown manifest schema_version"):
        score_feature_rows(manifest, [_row()])

    row = _row()
    row["schema_version"] = "feature_vector_row.v1"

    with pytest.raises(ValueError, match="unknown feature row schema_version"):
        score_feature_rows(_manifest(), [row])


def test_non_strict_json_constants_are_rejected(tmp_path: Path) -> None:
    manifest = tmp_path / "manifest.json"
    manifest.write_text('{"schema_version": NaN}', encoding="utf-8")

    with pytest.raises(ValueError, match="non-strict JSON constant"):
        load_manifest(manifest)


@pytest.mark.parametrize("bad_value", [float("nan"), float("inf"), True, "1.0"])
def test_feature_values_must_be_finite_numbers(bad_value: object) -> None:
    row = _row(
        features={
            "dns_failure_ratio": bad_value,
            "external_connection_count": 20,
        }
    )

    with pytest.raises(ValueError, match="finite number"):
        score_feature_rows(_manifest(), [row])


def test_missing_feature_column_is_rejected() -> None:
    row = _row(features={"dns_failure_ratio": 0.5})

    with pytest.raises(ValueError, match="missing"):
        score_feature_rows(_manifest(), [row])


def test_duplicate_feature_columns_are_rejected() -> None:
    manifest = _manifest(
        feature_columns=["dns_failure_ratio", "dns_failure_ratio"],
        weights=[1.0, 2.0],
    )

    with pytest.raises(ValueError, match="duplicates"):
        score_feature_rows(manifest, [_row()])


def test_unsafe_raw_identifiers_and_artifact_fields_are_rejected() -> None:
    with pytest.raises(ValueError, match="unsafe raw identifier|synthetic/coarse"):
        score_feature_rows(_manifest(), [_row(entity_id="host-alpha.example.com")])

    manifest = _manifest()
    manifest["training_data_summary"] = {
        "scope": "synthetic_fixture",
        "artifact_path": "model_artifact",
    }
    with pytest.raises(ValueError, match="forbidden raw field"):
        score_feature_rows(manifest, [_row()])

    manifest = _manifest(feature_columns=["payload_entropy"], weights=[1.0])
    with pytest.raises(ValueError, match="forbidden raw field"):
        score_feature_rows(
            manifest,
            [
                _row(
                    features={"payload_entropy": 5.0},
                )
            ],
        )


def test_unsupported_export_format_and_runtime_are_rejected() -> None:
    manifest = _manifest()
    manifest["export_format"] = "onnx_export"
    with pytest.raises(ValueError, match="unsupported export_format"):
        score_feature_rows(manifest, [_row()])

    manifest = _manifest()
    manifest["inference_runtime"] = "onnxruntime"
    with pytest.raises(ValueError, match="unsupported inference_runtime"):
        score_feature_rows(manifest, [_row()])


def test_directory_inputs_and_outputs_are_rejected(tmp_path: Path) -> None:
    with pytest.raises(ValueError, match="not a directory"):
        load_manifest(tmp_path)

    with pytest.raises(ValueError, match="not a directory"):
        dump_score_rows(score_feature_rows(_manifest(), [_row()]), tmp_path)
