from __future__ import annotations

import json
from contextlib import nullcontext
from pathlib import Path
from types import MappingProxyType

import pytest

from ares_netguard.models import time_series_forecast_evaluation as evaluation
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
from ares_netguard.models.time_series_residual import (
    generate_residual_report,
    load_time_window_rows,
)


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
    def __init__(self) -> None:
        self.contexts: list[tuple[float, ...]] = []

    def predict_quantiles(
        self,
        context: tuple[float, ...],
        *,
        prediction_length: int,
        quantile_levels: list[float],
    ) -> tuple[list[list[list[float]]], list[list[float]]]:
        self.contexts.append(tuple(context))
        assert prediction_length == 1
        assert quantile_levels == [0.1, 0.5, 0.9]
        median = sum(context[-16:]) / 16
        return [[[median - 3.0, median, median + 3.0]]], [[median]]


def _chronos_backend() -> tuple[ChronosBoltTinyLocalBackend, FakePipeline]:
    pipeline = FakePipeline()
    artifact = ForecastArtifactProvenance(
        model_id=CHRONOS_MODEL_ID,
        revision=CHRONOS_MODEL_REVISION,
        license_id="apache-2.0",
        serialization="safetensors",
        config_sha256=CHRONOS_CONFIG_SHA256,
        weights_sha256=CHRONOS_WEIGHTS_SHA256,
        bundle_sha256=CHRONOS_BUNDLE_SHA256,
        runtime_platform=CHRONOS_RUNTIME_PLATFORM,
        packages=MappingProxyType(dict(CHRONOS_PACKAGE_VERSIONS)),
    )
    return (
        ChronosBoltTinyLocalBackend(
            _pipeline=pipeline,
            _torch=FakeTorch(),
            artifact=artifact,
        ),
        pipeline,
    )


def _reports_and_labels() -> tuple[
    dict[str, object],
    dict[str, object],
    list[dict[str, object]],
    list[dict[str, object]],
]:
    rows = load_time_window_rows(
        Path("tests/fixtures/time_series_forecast/synthetic_windows.jsonl")
    )
    proxy = generate_residual_report(rows, history_window=64, calibration_window=32)
    backend, _pipeline = _chronos_backend()
    chronos = generate_residual_report(
        rows,
        history_window=64,
        calibration_window=32,
        backend=backend,
    )
    labels = evaluation.load_anomaly_labels(
        Path("tests/fixtures/time_series_forecast/anomaly_labels.jsonl")
    )
    return proxy, chronos, rows, labels


def test_evaluation_compares_aligned_reports_without_promotion_claims() -> None:
    proxy, chronos, rows, labels = _reports_and_labels()

    report = evaluation.generate_forecast_evaluation(proxy, chronos, rows, labels)

    assert report["schema_version"] == "time_series_forecast_evaluation.v0"
    assert report["dataset"] == {
        "cohort_id": "time_series_foundation_synthetic_v0",
        "cohort_sha256": evaluation.COHORT_SHA256,
        "series_count": 2,
        "observations_per_series": 128,
        "history_window": 64,
        "calibration_window": 32,
        "scored_observation_count": 64,
        "anomaly_label_count": 8,
        "labels_sent_to_backend": False,
    }
    assert [result["backend_id"] for result in report["backend_results"]] == [
        "chronos_bolt_tiny_local_v1",
        "rolling_mean_proxy_v1",
    ]
    assert all(result["count"] == 64 for result in report["backend_results"])
    assert all(0.0 <= result["auroc"] <= 1.0 for result in report["backend_results"])
    assert report["deltas"]["comparison"] == "chronos_minus_proxy"
    assert report["safety_flags"]["labels_sent_to_backend"] is False
    assert "not_model_promotion_gate" in report["non_claims"]


def test_labels_are_loaded_after_forecasts_and_never_enter_backend_context() -> None:
    rows = load_time_window_rows(
        Path("tests/fixtures/time_series_forecast/synthetic_windows.jsonl")
    )
    backend, pipeline = _chronos_backend()
    chronos = generate_residual_report(
        rows,
        history_window=64,
        calibration_window=32,
        backend=backend,
    )
    proxy = generate_residual_report(rows, history_window=64, calibration_window=32)
    labels = evaluation.load_anomaly_labels(
        Path("tests/fixtures/time_series_forecast/anomaly_labels.jsonl")
    )

    report = evaluation.generate_forecast_evaluation(proxy, chronos, rows, labels)

    assert len(pipeline.contexts) == 128
    assert all(len(context) == 64 for context in pipeline.contexts)
    assert all(all(isinstance(value, float) for value in context) for context in pipeline.contexts)
    assert report["safety_flags"]["reports_aligned"] is True


def test_evaluation_rejects_misaligned_actuals_and_unknown_labels() -> None:
    proxy, chronos, rows, labels = _reports_and_labels()
    misaligned = json.loads(json.dumps(chronos))
    misaligned["rows"][0]["actual_value"] += 1.0
    with pytest.raises(ValueError, match="frozen cohort actual values"):
        evaluation.generate_forecast_evaluation(proxy, misaligned, rows, labels)

    unknown = json.loads(json.dumps(labels))
    unknown[0]["window_start"] = "2027-01-01T00:00:00Z"
    unknown.sort(key=lambda row: (row["entity_id"], row["feature_name"], row["window_start"]))
    with pytest.raises(ValueError, match="pinned digest"):
        evaluation.generate_forecast_evaluation(proxy, chronos, rows, unknown)


def test_metric_ties_are_deterministic_and_average_precision_is_grouped() -> None:
    scores = [(0.9, True), (0.8, False), (0.7, True), (0.1, False)]

    assert evaluation._auroc(scores) == pytest.approx(0.75)
    assert evaluation._average_precision(scores) == pytest.approx(5.0 / 6.0)


def test_evaluation_validation_and_atomic_dump_preserve_existing_output(tmp_path: Path) -> None:
    proxy, chronos, rows, labels = _reports_and_labels()
    report = evaluation.generate_forecast_evaluation(proxy, chronos, rows, labels)
    output = tmp_path / "forecast-evaluation.json"
    output.write_text("existing\n", encoding="utf-8")
    tampered = json.loads(json.dumps(report))
    tampered["safety_flags"]["deployment_allowed"] = True

    with pytest.raises(ValueError, match="safety_flags are invalid"):
        evaluation.dump_forecast_evaluation(tampered, output)
    assert output.read_text(encoding="utf-8") == "existing\n"

    evaluation.dump_forecast_evaluation(report, output)
    assert json.loads(output.read_text(encoding="utf-8")) == report


def test_evaluation_rejects_tampered_delta_and_frozen_cohort() -> None:
    proxy, chronos, rows, labels = _reports_and_labels()
    report = evaluation.generate_forecast_evaluation(proxy, chronos, rows, labels)
    tampered_delta = json.loads(json.dumps(report))
    tampered_delta["deltas"]["mae"] = 999.0
    with pytest.raises(ValueError, match="delta mae is inconsistent"):
        evaluation.validate_forecast_evaluation(tampered_delta)

    tampered_rows = json.loads(json.dumps(rows))
    tampered_rows[0]["actual_value"] += 0.01
    with pytest.raises(ValueError, match="pinned digest"):
        evaluation.generate_forecast_evaluation(proxy, chronos, tampered_rows, labels)


def test_label_loader_rejects_non_strict_or_false_rows(tmp_path: Path) -> None:
    invalid = tmp_path / "labels.jsonl"
    invalid.write_text(
        '{"entity_id":"fixture-a","feature_name":"bytes_out",'
        '"window_start":"2026-01-01T00:00:00Z","is_anomaly":NaN}\n',
        encoding="utf-8",
    )
    with pytest.raises(ValueError, match="non-strict JSON constant"):
        evaluation.load_anomaly_labels(invalid)
