from __future__ import annotations

import json
from contextlib import nullcontext
from datetime import UTC, datetime, timedelta
from pathlib import Path
from types import MappingProxyType

import pytest

from ares_netguard.models import time_series_residual
from ares_netguard.models.time_series_forecast import (
    CHRONOS_BUNDLE_SHA256,
    CHRONOS_CONFIG_SHA256,
    CHRONOS_MODEL_ID,
    CHRONOS_MODEL_REVISION,
    CHRONOS_PACKAGE_VERSIONS,
    CHRONOS_RUNTIME_PLATFORM,
    CHRONOS_WEIGHTS_SHA256,
    OFFLINE_SYNTHETIC_BACKEND_SAFETY,
    ChronosBoltTinyLocalBackend,
    ForecastArtifactProvenance,
    ForecastBackendSafety,
    ForecastEstimate,
    ForecastRequest,
    RollingMeanProxyBackend,
)
from ares_netguard.models.time_series_residual import (
    LEGACY_REPORT_SCHEMA_VERSION,
    MODEL_FAMILY,
    MODEL_ID,
    PRETRAINED_REPORT_SCHEMA_VERSION,
    PROVENANCE_EVIDENCE_KIND,
    REPORT_SCHEMA_VERSION,
    dump_report,
    generate_residual_report,
    load_time_window_rows,
    residual_evidence_to_score_rows,
    validate_residual_report,
)


class _FakeChronosTorch:
    float32 = "float32"

    @staticmethod
    def tensor(values: object, *, dtype: object) -> object:
        assert dtype == "float32"
        return values

    @staticmethod
    def inference_mode() -> object:
        return nullcontext()


class _ContextQuantilePipeline:
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
        point = sum(context[-16:]) / 16
        return [[[point - 2.0, point, point + 2.0]]], [[point]]


def _fake_chronos_backend() -> tuple[ChronosBoltTinyLocalBackend, _ContextQuantilePipeline]:
    pipeline = _ContextQuantilePipeline()
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
            _torch=_FakeChronosTorch(),
            artifact=artifact,
        ),
        pipeline,
    )


def _series(
    values: list[float | int],
    *,
    entity_id: str = "host-a",
    feature_name: str = "bytes_out",
    start: datetime | None = None,
) -> list[dict[str, object]]:
    start = start or datetime(2026, 1, 1, tzinfo=UTC)
    return [
        {
            "entity_id": entity_id,
            "feature_name": feature_name,
            "window_start": (start + timedelta(minutes=5 * index))
            .isoformat()
            .replace("+00:00", "Z"),
            "actual_value": value,
        }
        for index, value in enumerate(values)
    ]


def _legacy_report() -> dict[str, object]:
    return {
        "schema_version": LEGACY_REPORT_SCHEMA_VERSION,
        "model_id": MODEL_ID,
        "model_family": MODEL_FAMILY,
        "history_window": 3,
        "interval_z": 2.0,
        "rows": [
            {
                "entity_id": "host-a",
                "feature_name": "bytes_out",
                "window_start": "2026-01-01T00:15:00Z",
                "actual_value": 18.0,
                "forecast_mean": 12.0,
                "forecast_lower": 8.734014,
                "forecast_upper": 15.265986,
                "residual": 6.0,
                "residual_z": 3.674235,
                "conformal_score": 0.75,
                "residual_risk": 0.918559,
                "model_id": MODEL_ID,
                "model_family": MODEL_FAMILY,
            }
        ],
    }


class RecordingBackend:
    backend_id = "recording_proxy_v1"
    backend_version = "1"
    backend_kind = "deterministic_test_proxy"
    settings = MappingProxyType({"mode": "recording"})
    safety = OFFLINE_SYNTHETIC_BACKEND_SAFETY

    def __init__(self) -> None:
        self.requests: list[ForecastRequest] = []

    def forecast_one(self, request: ForecastRequest) -> ForecastEstimate:
        self.requests.append(request)
        return RollingMeanProxyBackend().forecast_one(request)


class FixedBackend:
    backend_id = "fixed_proxy_v1"
    backend_version = "1"
    backend_kind = "deterministic_test_proxy"
    settings = MappingProxyType({"mode": "fixed"})
    safety = OFFLINE_SYNTHETIC_BACKEND_SAFETY

    def __init__(self, estimate: ForecastEstimate) -> None:
        self.estimate = estimate
        self.requests: list[ForecastRequest] = []

    def forecast_one(self, request: ForecastRequest) -> ForecastEstimate:
        self.requests.append(request)
        return self.estimate


def test_v1_uses_three_history_rows_eight_calibration_rows_and_emits_row_twelve() -> None:
    report = generate_residual_report(_series([0] * 11 + [10]))

    assert report["schema_version"] == REPORT_SCHEMA_VERSION
    assert report["history_window"] == 3
    assert report["calibration_window"] == 8
    assert report["forecast_backend"] == {
        "backend_id": "rolling_mean_proxy_v1",
        "backend_version": "1",
        "backend_kind": "deterministic_proxy",
        "settings": {
            "mean_method": "context_arithmetic_mean",
            "minimum_scale": 1e-6,
            "scale_method": "context_population_standard_deviation",
        },
    }
    assert report["calibration"] == {
        "method": "split_conformal_standardized_absolute_residual",
        "count": 8,
        "frozen": True,
        "tie_rule": ">=",
        "finite_sample_correction": True,
        "score_before_observe": True,
        "no_future_data": True,
    }
    assert all(report["safety_flags"].values())
    assert report["rows"] == [
        {
            "entity_id": "host-a",
            "feature_name": "bytes_out",
            "window_start": "2026-01-01T00:55:00Z",
            "actual_value": 10.0,
            "forecast_mean": 0.0,
            "forecast_lower": -0.000002,
            "forecast_upper": 0.000002,
            "residual": 10.0,
            "residual_z": 10000000.0,
            "conformal_score": 0.888889,
            "residual_risk": 1.0,
            "model_id": MODEL_ID,
            "model_family": MODEL_FAMILY,
        }
    ]


def test_v2_uses_fixed_chronos_run_and_emits_sanitized_pretrained_provenance() -> None:
    backend, pipeline = _fake_chronos_backend()
    rows = load_time_window_rows(
        Path("tests/fixtures/time_series_forecast/synthetic_windows.jsonl")
    )

    report = generate_residual_report(
        rows,
        history_window=64,
        calibration_window=32,
        interval_z=2.0,
        backend=backend,
    )

    assert report["schema_version"] == PRETRAINED_REPORT_SCHEMA_VERSION
    assert len(report["rows"]) == 64
    assert len(pipeline.contexts) == 128
    first_series = [
        float(row["actual_value"]) for row in rows if row["entity_id"] == "fixture-chronos-a"
    ]
    second_series = [
        float(row["actual_value"]) for row in rows if row["entity_id"] == "fixture-chronos-b"
    ]
    assert pipeline.contexts[0] == tuple(first_series[:64])
    assert pipeline.contexts[1] == tuple(first_series[1:65])
    assert pipeline.contexts[64] == tuple(second_series[:64])
    assert report["rows"][0]["window_start"] == "2026-02-01T01:36:00Z"
    assert report["forecast_backend"]["backend_id"] == "chronos_bolt_tiny_local_v1"
    assert report["forecast_backend"]["settings"]["point_method"] == "median_q0_5"
    assert report["forecast_backend"]["artifact"] == {
        "model_id": CHRONOS_MODEL_ID,
        "revision": CHRONOS_MODEL_REVISION,
        "license_id": "apache-2.0",
        "serialization": "safetensors",
        "config_sha256": CHRONOS_CONFIG_SHA256,
        "weights_sha256": CHRONOS_WEIGHTS_SHA256,
        "bundle_sha256": CHRONOS_BUNDLE_SHA256,
        "runtime_platform": CHRONOS_RUNTIME_PLATFORM,
        "packages": dict(CHRONOS_PACKAGE_VERSIONS),
    }
    assert report["safety_flags"] == {
        "local_only": True,
        "synthetic_only": True,
        "pretrained_model_used": True,
        "operator_provisioned_artifact": True,
        "artifact_digest_verified": True,
        "local_files_only": True,
        "network_used": False,
        "download_used": False,
        "external_service_used": False,
        "remote_code_used": False,
        "artifact_persisted_by_ares": False,
        "deployment_allowed": False,
    }
    assert "/tmp" not in json.dumps(report)


def test_v2_score_conversion_keeps_v0_score_rows_and_appends_one_provenance() -> None:
    backend, _pipeline = _fake_chronos_backend()
    report = generate_residual_report(
        load_time_window_rows(Path("tests/fixtures/time_series_forecast/synthetic_windows.jsonl")),
        history_window=64,
        calibration_window=32,
        interval_z=2.0,
        backend=backend,
    )

    score_rows = residual_evidence_to_score_rows(report)
    provenance = score_rows[0]["scores"][MODEL_ID]["evidence"][-1]

    assert {row["schema_version"] for row in score_rows} == {"model_score_row.v0"}
    assert provenance == {
        "evidence_kind": PROVENANCE_EVIDENCE_KIND,
        "forecast_backend": report["forecast_backend"],
        "calibration": report["calibration"],
        "safety_flags": report["safety_flags"],
    }


def test_v2_rejects_non_pinned_run_settings_before_inference() -> None:
    backend, pipeline = _fake_chronos_backend()
    rows = load_time_window_rows(
        Path("tests/fixtures/time_series_forecast/synthetic_windows.jsonl")
    )

    with pytest.raises(ValueError, match="history_window=64, calibration_window=32"):
        generate_residual_report(
            rows,
            history_window=32,
            calibration_window=32,
            interval_z=2.0,
            backend=backend,
        )
    assert pipeline.contexts == []


def test_v2_validator_rejects_artifact_and_safety_drift() -> None:
    backend, _pipeline = _fake_chronos_backend()
    report = generate_residual_report(
        load_time_window_rows(Path("tests/fixtures/time_series_forecast/synthetic_windows.jsonl")),
        history_window=64,
        calibration_window=32,
        interval_z=2.0,
        backend=backend,
    )
    artifact_drift = json.loads(json.dumps(report))
    artifact_drift["forecast_backend"]["artifact"]["revision"] = "main"
    with pytest.raises(ValueError, match="artifact.revision is not pinned"):
        validate_residual_report(artifact_drift)

    safety_drift = json.loads(json.dumps(report))
    safety_drift["safety_flags"]["network_used"] = True
    with pytest.raises(ValueError, match="network_used must be false"):
        validate_residual_report(safety_drift)

    settings_drift = json.loads(json.dumps(report))
    settings_drift["forecast_backend"]["settings"]["point_method"] = "context_arithmetic_mean"
    with pytest.raises(ValueError, match="settings must be pinned"):
        validate_residual_report(settings_drift)


def test_backend_receives_only_past_numeric_context_and_prefix_is_invariant() -> None:
    values = list(range(11)) + [99]
    backend = RecordingBackend()
    report = generate_residual_report(_series(values), backend=backend)

    assert len(backend.requests) == 9
    for call_index, request in enumerate(backend.requests):
        target_index = call_index + 3
        assert request.context == tuple(
            float(value) for value in values[target_index - 3 : target_index]
        )
        assert request.interval_z == 2.0
        assert all(isinstance(value, float) for value in request.context)
    assert report["rows"][0]["actual_value"] == 99.0

    extended_backend = RecordingBackend()
    extended = generate_residual_report(_series(values + [1000]), backend=extended_backend)
    assert extended["rows"][0] == report["rows"][0]
    assert extended_backend.requests[:9] == backend.requests


def test_calibration_is_frozen_and_uses_finite_sample_correction_with_ties() -> None:
    backend = FixedBackend(ForecastEstimate(mean=0.0, lower=-20.0, upper=20.0, scale=1.0))
    values = [0, 0, 0] + list(range(1, 9)) + [100, 8]

    report = generate_residual_report(_series(values), backend=backend)

    assert len(backend.requests) == 10
    assert [row["conformal_score"] for row in report["rows"]] == [0.888889, 0.777778]
    # The score of 100 is not added to the frozen calibration cohort. For the
    # second score of 8, one of the original eight calibration scores ties it:
    # p=(1+1)/9, anomaly score=7/9.
    assert report["rows"][1]["conformal_score"] == pytest.approx(7.0 / 9.0, abs=1e-6)


def test_interval_breach_floor_is_preserved() -> None:
    backend = FixedBackend(ForecastEstimate(mean=0.0, lower=-2.0, upper=2.0, scale=1.0))
    report = generate_residual_report(
        _series([0, 0, 0] + [10] * 8 + [2.1]),
        backend=backend,
    )

    row = report["rows"][0]
    assert row["conformal_score"] == 0.0
    assert abs(row["residual_z"]) / 4.0 < 0.75
    assert row["actual_value"] > row["forecast_upper"]
    assert row["residual_risk"] == 0.75


def test_series_state_is_independent_when_rows_are_interleaved() -> None:
    first = _series([0] * 11 + [5], entity_id="host-a")
    second = _series([10] * 11 + [20], entity_id="host-b")
    interleaved = [row for pair in zip(first, second, strict=True) for row in pair]

    report = generate_residual_report(interleaved)

    assert [(row["entity_id"], row["actual_value"]) for row in report["rows"]] == [
        ("host-a", 5.0),
        ("host-b", 20.0),
    ]


def test_v1_score_conversion_preserves_feature_rows_and_appends_one_provenance() -> None:
    rows = _series([0] * 11 + [5], feature_name="bytes_out")
    rows += _series([0] * 11 + [1], feature_name="dns_failure_ratio")
    report = generate_residual_report(rows)

    score_rows = residual_evidence_to_score_rows(report)
    evidence = score_rows[0]["scores"][MODEL_ID]["evidence"]

    assert [item.get("feature_name") for item in evidence[:-1]] == [
        "bytes_out",
        "dns_failure_ratio",
    ]
    assert evidence[-1] == {
        "evidence_kind": PROVENANCE_EVIDENCE_KIND,
        "forecast_backend": report["forecast_backend"],
        "calibration": report["calibration"],
    }
    assert score_rows[0]["schema_version"] == "model_score_row.v0"


def test_legacy_v0_is_strict_read_only_input_and_has_no_v1_provenance() -> None:
    legacy = _legacy_report()
    validate_residual_report(legacy)

    score_rows = residual_evidence_to_score_rows(legacy)

    assert score_rows[0]["scores"][MODEL_ID]["risk"] == 0.918559
    assert score_rows[0]["scores"][MODEL_ID]["evidence"] == legacy["rows"]


def test_short_series_is_rejected_before_backend_execution() -> None:
    backend = RecordingBackend()
    with pytest.raises(ValueError, match="requires at least 12 observations"):
        generate_residual_report(_series([0] * 11), backend=backend)
    assert backend.requests == []


@pytest.mark.parametrize("bad_value", [float("nan"), float("inf"), True, "42"])
def test_invalid_actual_value_is_rejected(bad_value: object) -> None:
    rows = _series([0] * 12)
    rows[0]["actual_value"] = bad_value
    with pytest.raises(ValueError, match="actual_value must be a finite number"):
        generate_residual_report(rows)


@pytest.mark.parametrize(
    "field, value, message",
    [
        ("entity_id", "10.0.0.1", "safe coarse identifier"),
        ("entity_id", "host-a.example", "safe coarse identifier"),
        ("feature_name", "BytesOut", "snake_case"),
        ("feature_name", "bytes-out", "snake_case"),
        ("window_start", "2026-01-01T00:00:00+09:00", "UTC timestamp"),
        ("window_start", "2026-01-01T00:00:00", "UTC timestamp"),
    ],
)
def test_unsafe_identifiers_and_non_utc_windows_are_rejected(
    field: str, value: object, message: str
) -> None:
    rows = _series([0] * 12)
    rows[0][field] = value
    with pytest.raises(ValueError, match=message):
        generate_residual_report(rows)


def test_unknown_time_window_fields_are_rejected() -> None:
    rows = _series([0] * 12)
    rows[0]["path"] = "/private/input.json"
    with pytest.raises(ValueError, match=r"unexpected \['path'\]"):
        generate_residual_report(rows)


def test_duplicate_and_unordered_windows_are_rejected() -> None:
    duplicate = _series([0] * 12)
    duplicate[1]["window_start"] = duplicate[0]["window_start"]
    with pytest.raises(ValueError, match="duplicate window_start"):
        generate_residual_report(duplicate)

    unordered = _series([0] * 12)
    unordered[2], unordered[3] = unordered[3], unordered[2]
    with pytest.raises(ValueError, match="strictly increasing"):
        generate_residual_report(unordered)


def test_strict_json_loader_rejects_constants_duplicate_keys_and_wrapper_fields(
    tmp_path: Path,
) -> None:
    malformed = tmp_path / "malformed.json"
    malformed.write_text("{", encoding="utf-8")
    with pytest.raises(json.JSONDecodeError):
        load_time_window_rows(malformed)

    invalid_constant = tmp_path / "constant.jsonl"
    invalid_constant.write_text(
        '{"entity_id":"host-a","feature_name":"bytes_out",'
        '"window_start":"2026-01-01T00:00:00Z","actual_value":NaN}\n',
        encoding="utf-8",
    )
    with pytest.raises(ValueError, match="non-strict JSON constant"):
        load_time_window_rows(invalid_constant)

    duplicate_key = tmp_path / "duplicate.json"
    duplicate_key.write_text('{"rows":[],"rows":[]}', encoding="utf-8")
    with pytest.raises(ValueError, match="duplicate JSON object key"):
        load_time_window_rows(duplicate_key)

    wrapper = tmp_path / "wrapper.json"
    wrapper.write_text('{"rows":[],"source":"private"}', encoding="utf-8")
    with pytest.raises(ValueError, match=r"unexpected \['source'\]"):
        load_time_window_rows(wrapper)


def test_oversized_input_is_rejected_before_row_processing() -> None:
    row = _series([0])[0]
    with pytest.raises(ValueError, match="must not exceed"):
        generate_residual_report([row] * (time_series_residual.MAX_INPUT_ROWS + 1))


@pytest.mark.parametrize(
    "estimate, message",
    [
        (ForecastEstimate(0.0, -1.0, 1.0, 0.0), "scale must be positive"),
        (ForecastEstimate(0.0, 1.0, 2.0, 1.0), "lower <= mean <= upper"),
        (ForecastEstimate(float("nan"), -1.0, 1.0, 1.0), "mean must be a finite"),
    ],
)
def test_malformed_backend_estimates_fail_without_fallback(
    estimate: ForecastEstimate, message: str
) -> None:
    backend = FixedBackend(estimate)
    with pytest.raises(ValueError, match=message):
        generate_residual_report(_series([0] * 12), backend=backend)
    assert len(backend.requests) == 1


def test_backend_without_safe_offline_contract_is_rejected_before_execution() -> None:
    backend = FixedBackend(ForecastEstimate(0.0, -1.0, 1.0, 1.0))
    backend.safety = ForecastBackendSafety(
        local_only=False,
        synthetic_only=True,
        no_pretrained_model=False,
        no_artifact=False,
        no_network=False,
        no_download=False,
        no_external_service=False,
        no_deployment=True,
    )

    with pytest.raises(ValueError, match="safety_flags.local_only must be true"):
        generate_residual_report(_series([0] * 12), backend=backend)
    assert backend.requests == []


def test_derived_calibration_overflow_is_rejected_before_freezing() -> None:
    backend = FixedBackend(ForecastEstimate(-1e308, -1e308, 0.0, 1.0))
    with pytest.raises(ValueError, match="forecast calibration residual must be a finite"):
        generate_residual_report(_series([0, 0, 0] + [1e308] * 9), backend=backend)
    assert len(backend.requests) == 1


def test_complete_report_validation_and_atomic_write_preserve_existing_output(
    tmp_path: Path,
) -> None:
    report = generate_residual_report(_series([0] * 11 + [5]))
    output = tmp_path / "residual-report.json"
    output.write_text("existing\n", encoding="utf-8")
    tampered = json.loads(json.dumps(report))
    tampered["calibration"]["frozen"] = False

    with pytest.raises(ValueError, match="calibration.frozen must be true"):
        dump_report(tampered, output)
    assert output.read_text(encoding="utf-8") == "existing\n"

    dump_report(report, output)
    assert json.loads(output.read_text(encoding="utf-8")) == report


def test_repository_output_path_is_restricted_to_runtime_roots(tmp_path: Path) -> None:
    report = generate_residual_report(_series([0] * 11 + [5]))
    forbidden = tmp_path / "docs" / "report.json"
    forbidden.parent.mkdir()
    with pytest.raises(ValueError, match="data/reports"):
        dump_report(report, forbidden, repo_root=tmp_path)

    allowed = tmp_path / "data" / "reports" / "report.json"
    allowed.parent.mkdir(parents=True)
    dump_report(report, allowed, repo_root=tmp_path)
    assert json.loads(allowed.read_text(encoding="utf-8")) == report


def test_forbidden_repository_symlink_output_cannot_bypass_path_policy(
    tmp_path: Path,
) -> None:
    report = generate_residual_report(_series([0] * 11 + [5]))
    repo = tmp_path / "repo"
    docs = repo / "docs"
    docs.mkdir(parents=True)
    outside = tmp_path / "outside.json"
    outside.write_text("outside\n", encoding="utf-8")
    output = docs / "report.json"
    output.symlink_to(outside)

    with pytest.raises(ValueError, match="must not be a symlink"):
        dump_report(report, output, repo_root=repo)

    assert output.is_symlink()
    assert outside.read_text(encoding="utf-8") == "outside\n"


def test_allowed_repository_root_cannot_escape_through_symlinked_parent(
    tmp_path: Path,
) -> None:
    report = generate_residual_report(_series([0] * 11 + [5]))
    repo = tmp_path / "repo"
    reports = repo / "data" / "reports"
    reports.mkdir(parents=True)
    outside = tmp_path / "outside"
    outside.mkdir()
    jump = reports / "jump"
    jump.symlink_to(outside, target_is_directory=True)
    output = jump / "report.json"

    with pytest.raises(ValueError, match="data/reports"):
        dump_report(report, output, repo_root=repo)

    assert not (outside / "report.json").exists()


def test_unknown_cli_backend_leaves_existing_output_untouched(tmp_path: Path) -> None:
    input_path = tmp_path / "rows.json"
    input_path.write_text(json.dumps(_series([0] * 12)), encoding="utf-8")
    output = tmp_path / "report.json"
    output.write_text("existing\n", encoding="utf-8")

    with pytest.raises(ValueError, match="unsupported forecast backend"):
        time_series_residual.main([str(input_path), str(output), "--backend", "organization/model"])

    assert output.read_text(encoding="utf-8") == "existing\n"


def test_chronos_artifact_failure_leaves_existing_output_untouched(tmp_path: Path) -> None:
    input_path = Path("tests/fixtures/time_series_forecast/synthetic_windows.jsonl")
    output = tmp_path / "report.json"
    output.write_text("existing\n", encoding="utf-8")

    with pytest.raises(ValueError, match="model root is unavailable"):
        time_series_residual.main(
            [
                str(input_path),
                str(output),
                "--backend",
                "chronos_bolt_tiny_local",
                "--model-root",
                str(tmp_path / "missing-model"),
                "--history-window",
                "64",
                "--calibration-window",
                "32",
            ]
        )

    assert output.read_text(encoding="utf-8") == "existing\n"
