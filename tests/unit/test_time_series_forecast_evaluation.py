from __future__ import annotations

import json
from contextlib import nullcontext
from pathlib import Path
from types import MappingProxyType

import pytest

from ares_netguard.models import time_series_forecast_evaluation as evaluation
from ares_netguard.models import time_series_foundation_smoke as foundation_smoke
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


def _replay_reports_and_labels() -> tuple[
    dict[str, object],
    dict[str, object],
    list[dict[str, object]],
    list[dict[str, object]],
    FakePipeline,
]:
    rows = load_time_window_rows(Path("tests/fixtures/time_series_forecast/replay_windows.jsonl"))
    proxy = generate_residual_report(rows, history_window=64, calibration_window=32)
    backend, pipeline = _chronos_backend()
    chronos = generate_residual_report(
        rows,
        history_window=64,
        calibration_window=32,
        backend=backend,
    )
    labels = evaluation.load_replay_anomaly_labels(
        Path("tests/fixtures/time_series_forecast/replay_anomaly_labels.jsonl")
    )
    return proxy, chronos, rows, labels, pipeline


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


def test_replay_evaluation_covers_all_regimes_and_stresses() -> None:
    proxy, chronos, rows, labels, pipeline = _replay_reports_and_labels()

    report = evaluation.generate_forecast_replay_evaluation(proxy, chronos, rows, labels)

    assert report["schema_version"] == "time_series_forecast_evaluation.v1"
    assert report["dataset"] == {
        "cohort_id": "time_series_foundation_drift_replay_synthetic_v1",
        "cohort_sha256": evaluation.REPLAY_COHORT_SHA256,
        "series_count": 2,
        "observations_per_series": 336,
        "history_window": 64,
        "calibration_window": 32,
        "cadence_seconds": 3600,
        "scored_observation_count": 480,
        "anomaly_label_count": 60,
        "regime_count": 6,
        "stress_case_count": 2,
        "labels_sent_to_backend": False,
    }
    assert [item["regime_id"] for item in report["regime_results"]] == [
        "stationary_reference",
        "abrupt_drift",
        "gradual_drift",
        "variance_expansion",
        "seasonality_change",
        "anomaly_density_change",
    ]
    assert [item["anomaly_label_count"] for item in report["regime_results"]] == [
        8,
        8,
        8,
        8,
        8,
        20,
    ]
    assert [item["anomaly_density"] for item in report["regime_results"]] == [
        0.1,
        0.1,
        0.1,
        0.1,
        0.1,
        0.25,
    ]
    assert all(item["scored_observation_count"] == 80 for item in report["regime_results"])
    assert all(result["count"] == 480 for result in report["aggregate"]["backend_results"])
    assert [item["observed_rejection"] for item in report["stress_results"]] == [
        "missing_observation_gap",
        "irregular_timestamp_interval",
    ]
    assert all(item["inference_call_count"] == 0 for item in report["stress_results"])
    assert len(pipeline.contexts) == 544
    evaluation.validate_forecast_evaluation(report)

    analytical = foundation_smoke._analytical_evidence(
        cohort_rows=rows,
        proxy_report=proxy,
        chronos_report=chronos,
        labels=labels,
        evaluation=report,
    )
    foundation_smoke.validate_analytical_evidence(analytical)
    assert analytical["proxy_residual_report"]["rows"] == proxy["rows"]
    assert analytical["chronos_residual_report"]["rows"] == chronos["rows"]
    assert analytical["anomaly_labels"] == labels

    tampered = json.loads(json.dumps(analytical))
    tampered["chronos_residual_report"]["rows"][0]["residual"] = float("inf")
    with pytest.raises(ValueError, match="finite"):
        foundation_smoke.validate_analytical_evidence(tampered)

    tampered = json.loads(json.dumps(analytical))
    regime = tampered["evaluation"]["regime_results"][0]
    regime["backend_results"][0]["mae"] += 0.1
    regime["deltas"]["mae"] = round(
        regime["backend_results"][0]["mae"] - regime["backend_results"][1]["mae"], 12
    )
    with pytest.raises(ValueError, match="stationary_reference.*metrics are inconsistent"):
        foundation_smoke.validate_analytical_evidence(tampered)

    tampered = json.loads(json.dumps(analytical))
    for report_name in ("proxy_residual_report", "chronos_residual_report"):
        residual_row = tampered[report_name]["rows"][0]
        for field in ("actual_value", "forecast_mean", "forecast_lower", "forecast_upper"):
            residual_row[field] += 12345.0
    with pytest.raises(ValueError, match="non-cohort actual values"):
        foundation_smoke.validate_analytical_evidence(tampered)

    tampered = json.loads(json.dumps(analytical))
    tampered["chronos_residual_report"]["rows"][0]["forecast_mean"] += 1.0
    with pytest.raises(ValueError, match="forecast and residual values are inconsistent"):
        foundation_smoke.validate_analytical_evidence(tampered)

    tampered = json.loads(json.dumps(analytical))
    tampered["cohort_rows"][0]["actual_value"] += 0.01
    with pytest.raises(ValueError, match="digest"):
        foundation_smoke.validate_analytical_evidence(tampered)


def test_replay_generic_loader_accepts_stress_shapes_but_replay_grid_rejects() -> None:
    _proxy, _chronos, rows, labels, _pipeline = _replay_reports_and_labels()
    normalized, _scored, _indexes = evaluation._validate_replay_cohort(rows, labels)

    results = evaluation.generate_replay_stress_results(normalized, labels)

    assert results[0]["observation_count"] == 671
    assert results[1]["observation_count"] == 672
    assert all(result["scoring_attempted"] is False for result in results)


def test_replay_rejects_digest_grid_metric_and_safety_drift() -> None:
    proxy, chronos, rows, labels, _pipeline = _replay_reports_and_labels()
    report = evaluation.generate_forecast_replay_evaluation(proxy, chronos, rows, labels)

    missing = rows[:150] + rows[151:]
    with pytest.raises(ValueError, match="missing_observation_gap"):
        evaluation.generate_forecast_replay_evaluation(proxy, chronos, missing, labels)

    irregular = json.loads(json.dumps(rows))
    irregular[150]["window_start"] = "2027-01-07T06:30:00Z"
    with pytest.raises(ValueError, match="irregular_timestamp_interval"):
        evaluation.generate_forecast_replay_evaluation(proxy, chronos, irregular, labels)

    metric_drift = json.loads(json.dumps(report))
    metric_drift["regime_results"][0]["deltas"]["mae"] += 0.1
    with pytest.raises(ValueError, match="delta mae is inconsistent"):
        evaluation.validate_forecast_evaluation(metric_drift)

    safety_drift = json.loads(json.dumps(report))
    safety_drift["safety_flags"]["stress_inference_used"] = True
    with pytest.raises(ValueError, match="safety_flags are invalid"):
        evaluation.validate_forecast_evaluation(safety_drift)


def test_replay_cli_flag_preserves_v0_default(monkeypatch: pytest.MonkeyPatch) -> None:
    calls: list[tuple[str, str]] = []

    monkeypatch.setattr(evaluation, "load_residual_report", lambda path: {"path": str(path)})
    monkeypatch.setattr(
        evaluation.time_series_residual,
        "load_time_window_rows",
        lambda path: [{"path": str(path)}],
    )
    monkeypatch.setattr(evaluation, "load_anomaly_labels", lambda path: [{"v0": str(path)}])
    monkeypatch.setattr(evaluation, "load_replay_anomaly_labels", lambda path: [{"v1": str(path)}])
    monkeypatch.setattr(
        evaluation,
        "generate_forecast_evaluation",
        lambda *_args: {"schema_version": evaluation.REPORT_SCHEMA_VERSION},
    )
    monkeypatch.setattr(
        evaluation,
        "generate_forecast_replay_evaluation",
        lambda *_args: {"schema_version": evaluation.REPLAY_REPORT_SCHEMA_VERSION},
    )
    monkeypatch.setattr(
        evaluation,
        "dump_forecast_evaluation",
        lambda report, path: calls.append((report["schema_version"], str(path))),
    )

    assert evaluation.main(["proxy", "chronos", "windows", "labels", "out-v0"]) == 0
    assert evaluation.main(["proxy", "chronos", "windows", "labels", "out-v1", "--replay"]) == 0
    assert calls == [
        (evaluation.REPORT_SCHEMA_VERSION, "out-v0"),
        (evaluation.REPLAY_REPORT_SCHEMA_VERSION, "out-v1"),
    ]
