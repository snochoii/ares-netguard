from __future__ import annotations

import json
from contextlib import nullcontext
from pathlib import Path
from types import MappingProxyType

import pytest

from ares_netguard.investigation.agentic_layer import generate_investigation_report
from ares_netguard.models.disagreement import generate_disagreement_report
from ares_netguard.models.evaluation_bundle import generate_evaluation_bundle
from ares_netguard.models.registry_metadata import generate_registry_metadata
from ares_netguard.models.score_row_composer import compose_score_rows
from ares_netguard.models.time_series_forecast import (
    CHRONOS_BUNDLE_SHA256,
    CHRONOS_CONFIG_SHA256,
    CHRONOS_MODEL_ID,
    CHRONOS_MODEL_REVISION,
    CHRONOS_PACKAGE_VERSIONS,
    CHRONOS_RUNTIME_PLATFORM,
    CHRONOS_WEIGHTS_SHA256,
    ChronosBoltTinyLocalBackend,
    ForecastArtifactProvenance,
)
from ares_netguard.models.time_series_forecast_evaluation import (
    generate_forecast_evaluation,
    load_anomaly_labels,
)
from ares_netguard.models.time_series_residual import (
    MODEL_ID,
    PRETRAINED_REPORT_SCHEMA_VERSION,
    generate_residual_report,
    load_time_window_rows,
)
from ares_netguard.storage.evidence_index import generate_evidence_index


class FakeTorch:
    float32 = "float32"

    @staticmethod
    def tensor(values: object, *, dtype: object) -> object:
        assert dtype == "float32"
        return values

    @staticmethod
    def inference_mode() -> object:
        return nullcontext()


class FakePipeline:
    def predict_quantiles(
        self,
        context: tuple[float, ...],
        *,
        prediction_length: int,
        quantile_levels: list[float],
    ) -> tuple[list[list[list[float]]], list[list[float]]]:
        assert prediction_length == 1
        assert quantile_levels == [0.1, 0.5, 0.9]
        median = sum(context[-16:]) / 16
        return [[[median - 3.0, median, median + 3.0]]], [[median]]


def _backend() -> ChronosBoltTinyLocalBackend:
    return ChronosBoltTinyLocalBackend(
        _pipeline=FakePipeline(),
        _torch=FakeTorch(),
        artifact=ForecastArtifactProvenance(
            model_id=CHRONOS_MODEL_ID,
            revision=CHRONOS_MODEL_REVISION,
            license_id="apache-2.0",
            serialization="safetensors",
            config_sha256=CHRONOS_CONFIG_SHA256,
            weights_sha256=CHRONOS_WEIGHTS_SHA256,
            bundle_sha256=CHRONOS_BUNDLE_SHA256,
            runtime_platform=CHRONOS_RUNTIME_PLATFORM,
            packages=MappingProxyType(dict(CHRONOS_PACKAGE_VERSIONS)),
        ),
    )


def test_v2_fixture_flows_through_comparison_and_existing_consumers() -> None:
    rows = load_time_window_rows(
        Path("tests/fixtures/time_series_forecast/synthetic_windows.jsonl")
    )
    labels = load_anomaly_labels(Path("tests/fixtures/time_series_forecast/anomaly_labels.jsonl"))
    proxy_report = generate_residual_report(rows, history_window=64, calibration_window=32)
    chronos_report = generate_residual_report(
        rows,
        history_window=64,
        calibration_window=32,
        backend=_backend(),
    )
    forecast_evaluation = generate_forecast_evaluation(
        proxy_report,
        chronos_report,
        rows,
        labels,
    )

    assert chronos_report["schema_version"] == PRETRAINED_REPORT_SCHEMA_VERSION
    score_rows = compose_score_rows(residual_reports=[chronos_report])
    assert {row["schema_version"] for row in score_rows} == {"model_score_row.v0"}
    assert all(MODEL_ID in row["scores"] for row in score_rows)

    disagreement = generate_disagreement_report(score_rows)
    investigation = generate_investigation_report(
        disagreement,
        evidence_reports=[chronos_report],
    )
    assert investigation["evidence_report_schemas"] == [PRETRAINED_REPORT_SCHEMA_VERSION]

    bundle = generate_evaluation_bundle(
        [chronos_report, forecast_evaluation, score_rows, disagreement, investigation]
    )
    schemas = bundle["aggregate_summary"]["schemas_present"]
    assert PRETRAINED_REPORT_SCHEMA_VERSION in schemas
    assert "time_series_forecast_evaluation.v0" in schemas

    registry = generate_registry_metadata(bundle)
    residual_entry = next(entry for entry in registry["entries"] if entry["model_id"] == MODEL_ID)
    assert PRETRAINED_REPORT_SCHEMA_VERSION in residual_entry["observed_source_schemas"]

    index = generate_evidence_index(
        [chronos_report, forecast_evaluation, score_rows, disagreement, investigation, registry]
    )
    indexed_schemas = index["aggregate_summary"]["schemas_present"]
    assert PRETRAINED_REPORT_SCHEMA_VERSION in indexed_schemas
    assert "time_series_forecast_evaluation.v0" in indexed_schemas

    tampered_v2 = json.loads(json.dumps(chronos_report))
    tampered_v2["safety_flags"]["network_used"] = True
    with pytest.raises(ValueError, match="safety_flags.network_used must be false"):
        generate_investigation_report(
            disagreement,
            evidence_reports=[tampered_v2],
        )
