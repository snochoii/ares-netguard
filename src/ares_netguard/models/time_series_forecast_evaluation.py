"""Held-out comparison for proxy and local pretrained forecast residual reports."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import re
from collections import Counter
from collections.abc import Mapping, Sequence
from datetime import datetime, timedelta
from pathlib import Path
from typing import Any

from ares_netguard.models import time_series_residual
from ares_netguard.models.time_series_forecast import (
    CHRONOS_BACKEND_ID,
    ROLLING_MEAN_PROXY_BACKEND_ID,
)

REPORT_SCHEMA_VERSION = "time_series_forecast_evaluation.v0"
REPLAY_REPORT_SCHEMA_VERSION = "time_series_forecast_evaluation.v1"
SUPPORTED_REPORT_SCHEMA_VERSIONS = frozenset({REPORT_SCHEMA_VERSION, REPLAY_REPORT_SCHEMA_VERSION})
EVALUATION_SCOPE = "local_synthetic_offline_forecast_backend_comparison"
COHORT_ID = "time_series_foundation_synthetic_v0"
COHORT_SHA256 = "af2a440076123497f1435ac477df200280767c1de7f07522dc58909ba4d6ade3"
EXPECTED_SERIES_COUNT = 2
EXPECTED_OBSERVATIONS_PER_SERIES = 128
EXPECTED_SCORED_PER_SERIES = 32
EXPECTED_SCORED_COUNT = EXPECTED_SERIES_COUNT * EXPECTED_SCORED_PER_SERIES
EXPECTED_ANOMALY_COUNT = 8
RISK_THRESHOLD = 0.75

REPORT_FIELDS = frozenset(
    {
        "schema_version",
        "evaluation_scope",
        "dataset",
        "backend_results",
        "deltas",
        "safety_flags",
        "non_claims",
    }
)
DATASET_FIELDS = frozenset(
    {
        "cohort_id",
        "cohort_sha256",
        "series_count",
        "observations_per_series",
        "history_window",
        "calibration_window",
        "scored_observation_count",
        "anomaly_label_count",
        "labels_sent_to_backend",
    }
)
RESULT_FIELDS = frozenset(
    {
        "backend_id",
        "report_schema",
        "point_method",
        "count",
        "mae",
        "rmse",
        "interval_coverage",
        "mean_interval_width",
        "auroc",
        "average_precision",
        "recall_at_0_75",
        "fpr_at_0_75",
    }
)
METRIC_FIELDS = (
    "mae",
    "rmse",
    "interval_coverage",
    "mean_interval_width",
    "auroc",
    "average_precision",
    "recall_at_0_75",
    "fpr_at_0_75",
)
DELTA_FIELDS = frozenset({"comparison", *METRIC_FIELDS})
SAFETY_FIELDS = frozenset(
    {
        "local_only",
        "synthetic_only",
        "reports_aligned",
        "labels_sent_to_backend",
        "no_future_data",
        "external_services_used",
        "deployment_allowed",
    }
)
LABEL_FIELDS = frozenset({"entity_id", "feature_name", "window_start", "is_anomaly"})
NON_CLAIMS = [
    "not_model_promotion_gate",
    "not_production_benchmark",
    "not_private_telemetry_evaluation",
    "not_external_service_evaluation",
    "not_deployment_approval",
    "not_native_runtime_execution",
]

REPLAY_EVALUATION_SCOPE = "local_synthetic_offline_forecast_drift_replay_comparison"
REPLAY_COHORT_ID = "time_series_foundation_drift_replay_synthetic_v1"
REPLAY_COHORT_SHA256 = "588231a1a3f0129aa680bdf87d2071118fa2336274e8bd1979a4ac72b4cda7a7"
REPLAY_SERIES = (("fixture-a", "bytes_out"), ("fixture-b", "connection_rate"))
REPLAY_OBSERVATIONS_PER_SERIES = 336
REPLAY_HISTORY_WINDOW = 64
REPLAY_CALIBRATION_WINDOW = 32
REPLAY_SCORING_OFFSET = REPLAY_HISTORY_WINDOW + REPLAY_CALIBRATION_WINDOW
REPLAY_SCORED_PER_SERIES = 240
REPLAY_SCORED_COUNT = 480
REPLAY_ANOMALY_COUNT = 60
REPLAY_CADENCE_SECONDS = 3600
REPLAY_START = datetime.fromisoformat("2027-01-01T00:00:00+00:00")
REPLAY_REGIMES = (
    {
        "regime_id": "stationary_reference",
        "start_index": 96,
        "end_index": 135,
        "scored_observations_per_series": 40,
        "anomaly_offsets": [7, 17, 29, 37],
        "anomaly_density": 0.1,
        "input_sha256": "a87d871497287bac7f7bdf0a167d1c6b860a61561f58366e58fca6e4d83ce31a",
    },
    {
        "regime_id": "abrupt_drift",
        "start_index": 136,
        "end_index": 175,
        "scored_observations_per_series": 40,
        "anomaly_offsets": [7, 17, 29, 37],
        "anomaly_density": 0.1,
        "input_sha256": "118a44658e5b14729a06fd56d5c9d25160a0fb8cb14459caa3999946800b3381",
    },
    {
        "regime_id": "gradual_drift",
        "start_index": 176,
        "end_index": 215,
        "scored_observations_per_series": 40,
        "anomaly_offsets": [7, 17, 29, 37],
        "anomaly_density": 0.1,
        "input_sha256": "14d015c502abc7240b9ea36c9cba340e774c0b44fb4b417ad016176978d91639",
    },
    {
        "regime_id": "variance_expansion",
        "start_index": 216,
        "end_index": 255,
        "scored_observations_per_series": 40,
        "anomaly_offsets": [7, 17, 29, 37],
        "anomaly_density": 0.1,
        "input_sha256": "c1fbef59ed416f6d3e8e3a430cec4246fb68f9360732d92d7e1f1a6ba1c33240",
    },
    {
        "regime_id": "seasonality_change",
        "start_index": 256,
        "end_index": 295,
        "scored_observations_per_series": 40,
        "anomaly_offsets": [7, 17, 29, 37],
        "anomaly_density": 0.1,
        "input_sha256": "969247467ad3e3ce3bd56d81fd91d2affba3842f939e155f892ffb3045af1c6d",
    },
    {
        "regime_id": "anomaly_density_change",
        "start_index": 296,
        "end_index": 335,
        "scored_observations_per_series": 40,
        "anomaly_offsets": [1, 5, 9, 13, 17, 21, 25, 29, 33, 37],
        "anomaly_density": 0.25,
        "input_sha256": "e9965032f1ead00e9c1adb77d06c51197e02a2394793d29639fafe2f64c308ed",
    },
)
REPLAY_STRESS_CASES = (
    {
        "stress_case_id": "missing_observation",
        "input_sha256": "c3ce59e87bbb5941ca37eeac8d1e55c1e918901ef1808a2a240b40b3f43ec0c8",
        "observation_count": 671,
        "expected_rejection": "missing_observation_gap",
    },
    {
        "stress_case_id": "irregular_timestamp_interval",
        "input_sha256": "58a66e89bf67215b4347690f2e581b108e64679038b738426ee73bf2f035ec15",
        "observation_count": 672,
        "expected_rejection": "irregular_timestamp_interval",
    },
)

REPLAY_REPORT_FIELDS = frozenset(
    {
        "schema_version",
        "evaluation_scope",
        "dataset",
        "regime_results",
        "stress_results",
        "aggregate",
        "safety_flags",
        "non_claims",
    }
)
REPLAY_DATASET_FIELDS = frozenset(
    {
        "cohort_id",
        "cohort_sha256",
        "series_count",
        "observations_per_series",
        "history_window",
        "calibration_window",
        "cadence_seconds",
        "scored_observation_count",
        "anomaly_label_count",
        "regime_count",
        "stress_case_count",
        "labels_sent_to_backend",
    }
)
REPLAY_REGIME_RESULT_FIELDS = frozenset(
    {
        "regime_id",
        "input_sha256",
        "start_index",
        "end_index",
        "scored_observation_count",
        "anomaly_label_count",
        "anomaly_density",
        "backend_results",
        "deltas",
    }
)
REPLAY_STRESS_RESULT_FIELDS = frozenset(
    {
        "stress_case_id",
        "input_sha256",
        "observation_count",
        "input_label_count",
        "scoring_attempted",
        "inference_call_count",
        "expected_rejection",
        "observed_rejection",
    }
)
REPLAY_AGGREGATE_FIELDS = frozenset({"backend_results", "deltas"})
REPLAY_SAFETY_FIELDS = frozenset(
    {
        "local_only",
        "synthetic_only",
        "reports_aligned",
        "labels_sent_to_backend",
        "no_future_data",
        "regular_grid_required",
        "calibration_frozen",
        "stress_inference_used",
        "external_services_used",
        "deployment_allowed",
    }
)
REPLAY_NON_CLAIMS = [
    "not_model_promotion_gate",
    "not_production_benchmark",
    "not_generalization_claim",
    "not_statistical_significance_claim",
    "not_model_superiority_claim",
    "not_export_readiness_decision",
    "not_model_or_dependency_repin",
    "not_private_telemetry_evaluation",
    "not_external_service_evaluation",
    "not_deployment_approval",
    "not_native_runtime_execution",
]

SAFE_ENTITY_ID_RE = re.compile(r"^(?:asset|entity|fixture|host|sensor)-[a-z0-9][a-z0-9_-]{0,62}$")
SAFE_FEATURE_NAME_RE = re.compile(r"^[a-z][a-z0-9_]{0,63}$")

JsonMap = dict[str, Any]
RowKey = tuple[str, str, str]


def load_residual_report(path: str | Path) -> JsonMap:
    source = Path(path)
    if source.is_dir():
        raise ValueError("residual evaluation source must be a file")
    payload = _loads_strict(source.read_text(encoding="utf-8"))
    if not isinstance(payload, Mapping):
        raise ValueError("residual evaluation source must be an object")
    report = dict(payload)
    time_series_residual.validate_residual_report(report)
    return report


def load_anomaly_labels(path: str | Path) -> list[JsonMap]:
    source = Path(path)
    if source.is_dir():
        raise ValueError("forecast evaluation labels must be a file")
    text = source.read_text(encoding="utf-8").strip()
    if not text:
        raise ValueError("forecast evaluation labels must not be empty")
    if source.suffix.lower() == ".jsonl":
        raw_rows = [_loads_strict(line) for line in text.splitlines() if line.strip()]
    else:
        raw_rows = _loads_strict(text)
    if not isinstance(raw_rows, list):
        raise ValueError("forecast evaluation labels must be a JSON list or JSONL rows")
    if len(raw_rows) != EXPECTED_ANOMALY_COUNT:
        raise ValueError(f"forecast evaluation requires {EXPECTED_ANOMALY_COUNT} anomaly labels")
    labels = [_validate_label(row) for row in raw_rows]
    keys = [_row_key(row) for row in labels]
    if keys != sorted(keys) or len(keys) != len(set(keys)):
        raise ValueError("forecast evaluation labels must be sorted and unique")
    return labels


def load_replay_anomaly_labels(path: str | Path) -> list[JsonMap]:
    """Load the exact anomaly-only label set for the frozen replay cohort."""
    source = Path(path)
    if source.is_dir():
        raise ValueError("replay evaluation labels must be a file")
    text = source.read_text(encoding="utf-8").strip()
    if not text:
        raise ValueError("replay evaluation labels must not be empty")
    raw_rows = (
        [_loads_strict(line) for line in text.splitlines() if line.strip()]
        if source.suffix.lower() == ".jsonl"
        else _loads_strict(text)
    )
    if not isinstance(raw_rows, list) or len(raw_rows) != REPLAY_ANOMALY_COUNT:
        raise ValueError(f"replay evaluation requires {REPLAY_ANOMALY_COUNT} anomaly labels")
    labels = [_validate_label(row) for row in raw_rows]
    keys = [_row_key(row) for row in labels]
    if keys != sorted(keys) or len(keys) != len(set(keys)):
        raise ValueError("replay evaluation labels must be sorted and unique")
    return labels


def generate_forecast_evaluation(
    proxy_report: Mapping[str, Any],
    chronos_report: Mapping[str, Any],
    cohort_rows: Sequence[Mapping[str, Any]],
    anomaly_labels: Sequence[Mapping[str, Any]],
) -> JsonMap:
    """Compare aligned residual reports without exposing labels to either backend."""
    time_series_residual.validate_residual_report(proxy_report)
    time_series_residual.validate_residual_report(chronos_report)
    _validate_expected_report(proxy_report, pretrained=False)
    _validate_expected_report(chronos_report, pretrained=True)

    labels = [_validate_label(row) for row in anomaly_labels]
    label_keys = [_row_key(row) for row in labels]
    if label_keys != sorted(label_keys) or len(label_keys) != len(set(label_keys)):
        raise ValueError("forecast evaluation labels must be sorted and unique")
    cohort_scored_rows = _validate_frozen_cohort(cohort_rows, labels)

    proxy_rows = _rows_by_key(proxy_report)
    chronos_rows = _rows_by_key(chronos_report)
    if list(proxy_rows) != list(chronos_rows):
        raise ValueError("forecast evaluation reports must contain identical ordered row keys")
    if list(proxy_rows) != list(cohort_scored_rows):
        raise ValueError("forecast evaluation reports must match the frozen cohort scoring rows")
    for key in proxy_rows:
        cohort_actual = cohort_scored_rows[key]["actual_value"]
        if (
            proxy_rows[key]["actual_value"] != chronos_rows[key]["actual_value"]
            or proxy_rows[key]["actual_value"] != cohort_actual
        ):
            raise ValueError(
                "forecast evaluation reports must contain the frozen cohort actual values"
            )

    if len(label_keys) != EXPECTED_ANOMALY_COUNT or not set(label_keys) <= set(proxy_rows):
        raise ValueError("forecast evaluation labels must identify eight aligned scored rows")

    series_counts = Counter((key[0], key[1]) for key in proxy_rows)
    if len(series_counts) != EXPECTED_SERIES_COUNT or set(series_counts.values()) != {
        EXPECTED_SCORED_PER_SERIES
    }:
        raise ValueError("forecast evaluation requires two series with 32 scored rows each")

    anomaly_keys = set(label_keys)
    proxy_result = _backend_metrics(proxy_report, proxy_rows, anomaly_keys)
    chronos_result = _backend_metrics(chronos_report, chronos_rows, anomaly_keys)
    results = sorted([proxy_result, chronos_result], key=lambda item: item["backend_id"])
    deltas = {
        "comparison": "chronos_minus_proxy",
        **{field: _round(chronos_result[field] - proxy_result[field]) for field in METRIC_FIELDS},
    }
    report = {
        "schema_version": REPORT_SCHEMA_VERSION,
        "evaluation_scope": EVALUATION_SCOPE,
        "dataset": {
            "cohort_id": COHORT_ID,
            "cohort_sha256": COHORT_SHA256,
            "series_count": EXPECTED_SERIES_COUNT,
            "observations_per_series": EXPECTED_OBSERVATIONS_PER_SERIES,
            "history_window": 64,
            "calibration_window": 32,
            "scored_observation_count": EXPECTED_SCORED_COUNT,
            "anomaly_label_count": EXPECTED_ANOMALY_COUNT,
            "labels_sent_to_backend": False,
        },
        "backend_results": results,
        "deltas": deltas,
        "safety_flags": {
            "local_only": True,
            "synthetic_only": True,
            "reports_aligned": True,
            "labels_sent_to_backend": False,
            "no_future_data": True,
            "external_services_used": False,
            "deployment_allowed": False,
        },
        "non_claims": list(NON_CLAIMS),
    }
    validate_forecast_evaluation(report)
    return report


def generate_forecast_replay_evaluation(
    proxy_report: Mapping[str, Any],
    chronos_report: Mapping[str, Any],
    cohort_rows: Sequence[Mapping[str, Any]],
    anomaly_labels: Sequence[Mapping[str, Any]],
) -> JsonMap:
    """Compare aligned reports across the frozen long replay and rejection stresses."""
    time_series_residual.validate_residual_report(proxy_report)
    time_series_residual.validate_residual_report(chronos_report)
    _validate_replay_expected_report(proxy_report, pretrained=False)
    _validate_replay_expected_report(chronos_report, pretrained=True)

    labels = [_validate_label(row) for row in anomaly_labels]
    label_keys = [_row_key(row) for row in labels]
    if len(labels) != REPLAY_ANOMALY_COUNT:
        raise ValueError(f"replay evaluation requires {REPLAY_ANOMALY_COUNT} anomaly labels")
    if label_keys != sorted(label_keys) or len(label_keys) != len(set(label_keys)):
        raise ValueError("replay evaluation labels must be sorted and unique")

    normalized, scored_rows, index_by_key = _validate_replay_cohort(cohort_rows, labels)
    stress_results = generate_replay_stress_results(normalized, labels)
    proxy_rows = _rows_by_key(proxy_report)
    chronos_rows = _rows_by_key(chronos_report)
    expected_keys = list(scored_rows)
    if list(proxy_rows) != list(chronos_rows) or list(proxy_rows) != expected_keys:
        raise ValueError("replay reports must contain identical frozen scoring-row keys")
    for key in expected_keys:
        actual = scored_rows[key]["actual_value"]
        if proxy_rows[key]["actual_value"] != actual or chronos_rows[key]["actual_value"] != actual:
            raise ValueError("replay reports must contain the frozen cohort actual values")
    if not set(label_keys) <= set(scored_rows):
        raise ValueError("replay labels must identify frozen scored rows")

    anomaly_keys = set(label_keys)
    regime_results = []
    for regime in REPLAY_REGIMES:
        start = int(regime["start_index"])
        end = int(regime["end_index"])
        keys = [key for key in expected_keys if start <= index_by_key[key] <= end]
        proxy_subset = {key: proxy_rows[key] for key in keys}
        chronos_subset = {key: chronos_rows[key] for key in keys}
        regime_anomalies = anomaly_keys & set(keys)
        proxy_result = _backend_metrics(proxy_report, proxy_subset, regime_anomalies)
        chronos_result = _backend_metrics(chronos_report, chronos_subset, regime_anomalies)
        results = sorted([proxy_result, chronos_result], key=lambda item: item["backend_id"])
        regime_results.append(
            {
                "regime_id": regime["regime_id"],
                "input_sha256": regime["input_sha256"],
                "start_index": start,
                "end_index": end,
                "scored_observation_count": len(keys),
                "anomaly_label_count": len(regime_anomalies),
                "anomaly_density": regime["anomaly_density"],
                "backend_results": results,
                "deltas": _metric_deltas(proxy_result, chronos_result),
            }
        )

    proxy_aggregate = _backend_metrics(proxy_report, proxy_rows, anomaly_keys)
    chronos_aggregate = _backend_metrics(chronos_report, chronos_rows, anomaly_keys)
    aggregate_results = sorted(
        [proxy_aggregate, chronos_aggregate], key=lambda item: item["backend_id"]
    )
    report = {
        "schema_version": REPLAY_REPORT_SCHEMA_VERSION,
        "evaluation_scope": REPLAY_EVALUATION_SCOPE,
        "dataset": {
            "cohort_id": REPLAY_COHORT_ID,
            "cohort_sha256": REPLAY_COHORT_SHA256,
            "series_count": len(REPLAY_SERIES),
            "observations_per_series": REPLAY_OBSERVATIONS_PER_SERIES,
            "history_window": REPLAY_HISTORY_WINDOW,
            "calibration_window": REPLAY_CALIBRATION_WINDOW,
            "cadence_seconds": REPLAY_CADENCE_SECONDS,
            "scored_observation_count": REPLAY_SCORED_COUNT,
            "anomaly_label_count": REPLAY_ANOMALY_COUNT,
            "regime_count": len(REPLAY_REGIMES),
            "stress_case_count": len(REPLAY_STRESS_CASES),
            "labels_sent_to_backend": False,
        },
        "regime_results": regime_results,
        "stress_results": stress_results,
        "aggregate": {
            "backend_results": aggregate_results,
            "deltas": _metric_deltas(proxy_aggregate, chronos_aggregate),
        },
        "safety_flags": {
            "local_only": True,
            "synthetic_only": True,
            "reports_aligned": True,
            "labels_sent_to_backend": False,
            "no_future_data": True,
            "regular_grid_required": True,
            "calibration_frozen": True,
            "stress_inference_used": False,
            "external_services_used": False,
            "deployment_allowed": False,
        },
        "non_claims": list(REPLAY_NON_CLAIMS),
    }
    validate_forecast_evaluation(report)
    return report


def generate_replay_stress_results(
    normalized_rows: Sequence[Mapping[str, Any]],
    labels: Sequence[Mapping[str, Any]],
) -> list[JsonMap]:
    """Derive and prove the two unsupported cadence cases fail before inference."""
    rows = [
        {
            "entity_id": row["entity_id"],
            "feature_name": row["feature_name"],
            "window_start": row["window_start"],
            "actual_value": row["actual_value"],
        }
        for row in normalized_rows
    ]
    missing_rows = [
        row
        for row in rows
        if not (
            row["entity_id"] == "fixture-a"
            and row["feature_name"] == "bytes_out"
            and row["window_start"]
            == (REPLAY_START + timedelta(hours=150)).isoformat().replace("+00:00", "Z")
        )
    ]
    irregular_rows = [dict(row) for row in rows]
    irregular_key = (
        "fixture-b",
        "connection_rate",
        (REPLAY_START + timedelta(hours=150)).isoformat().replace("+00:00", "Z"),
    )
    for row in irregular_rows:
        if _row_key(row) == irregular_key:
            row["window_start"] = (
                (REPLAY_START + timedelta(hours=150, minutes=30)).isoformat().replace("+00:00", "Z")
            )
            break

    cases = ((REPLAY_STRESS_CASES[0], missing_rows), (REPLAY_STRESS_CASES[1], irregular_rows))
    results = []
    for contract, mutated_rows in cases:
        expected = str(contract["expected_rejection"])
        observed = _replay_grid_rejection(mutated_rows)
        if observed != expected:
            raise ValueError(f"replay stress {contract['stress_case_id']} rejection drifted")
        digest = _stress_sha256(
            stress_case_id=str(contract["stress_case_id"]),
            rows=mutated_rows,
            labels=labels,
            expected_rejection=expected,
        )
        if digest != contract["input_sha256"]:
            raise ValueError(f"replay stress {contract['stress_case_id']} digest drifted")
        results.append(
            {
                "stress_case_id": contract["stress_case_id"],
                "input_sha256": digest,
                "observation_count": len(mutated_rows),
                "input_label_count": len(labels),
                "scoring_attempted": False,
                "inference_call_count": 0,
                "expected_rejection": expected,
                "observed_rejection": observed,
            }
        )
    return results


def validate_forecast_evaluation(report: Mapping[str, Any]) -> None:
    if not isinstance(report, Mapping):
        raise ValueError("forecast evaluation must be an object")
    if report.get("schema_version") == REPLAY_REPORT_SCHEMA_VERSION:
        _validate_forecast_replay_evaluation(report)
        return
    _require_exact_fields(report, REPORT_FIELDS, "forecast evaluation")
    if report["schema_version"] != REPORT_SCHEMA_VERSION:
        raise ValueError(f"forecast evaluation schema must be '{REPORT_SCHEMA_VERSION}'")
    if report["evaluation_scope"] != EVALUATION_SCOPE:
        raise ValueError("forecast evaluation scope is invalid")

    dataset = report["dataset"]
    if not isinstance(dataset, Mapping):
        raise ValueError("forecast evaluation dataset must be an object")
    _require_exact_fields(dataset, DATASET_FIELDS, "forecast evaluation dataset")
    expected_dataset = {
        "cohort_id": COHORT_ID,
        "cohort_sha256": COHORT_SHA256,
        "series_count": EXPECTED_SERIES_COUNT,
        "observations_per_series": EXPECTED_OBSERVATIONS_PER_SERIES,
        "history_window": 64,
        "calibration_window": 32,
        "scored_observation_count": EXPECTED_SCORED_COUNT,
        "anomaly_label_count": EXPECTED_ANOMALY_COUNT,
        "labels_sent_to_backend": False,
    }
    if dict(dataset) != expected_dataset:
        raise ValueError("forecast evaluation dataset must match the frozen cohort")

    results = report["backend_results"]
    if not isinstance(results, list) or len(results) != 2:
        raise ValueError("forecast evaluation requires exactly two backend results")
    expected_pairs = [
        (CHRONOS_BACKEND_ID, time_series_residual.PRETRAINED_REPORT_SCHEMA_VERSION),
        (ROLLING_MEAN_PROXY_BACKEND_ID, time_series_residual.REPORT_SCHEMA_VERSION),
    ]
    actual_pairs = []
    for result in results:
        if not isinstance(result, Mapping):
            raise ValueError("forecast evaluation backend results must be objects")
        _require_exact_fields(result, RESULT_FIELDS, "forecast evaluation backend result")
        actual_pairs.append((result["backend_id"], result["report_schema"]))
        expected_point_method = (
            "median_q0_5"
            if result["backend_id"] == CHRONOS_BACKEND_ID
            else "context_arithmetic_mean"
        )
        if result["point_method"] != expected_point_method:
            raise ValueError("forecast evaluation point method is not pinned to its backend")
        if result["count"] != EXPECTED_SCORED_COUNT:
            raise ValueError("forecast evaluation backend count must be 64")
        _non_negative_number(result["mae"], "mae")
        _non_negative_number(result["rmse"], "rmse")
        _non_negative_number(result["mean_interval_width"], "mean_interval_width")
        for field in (
            "interval_coverage",
            "auroc",
            "average_precision",
            "recall_at_0_75",
            "fpr_at_0_75",
        ):
            _bounded_number(result[field], field, 0.0, 1.0)
    if actual_pairs != expected_pairs:
        raise ValueError("forecast evaluation backend results must be sorted and pinned")

    deltas = report["deltas"]
    if not isinstance(deltas, Mapping):
        raise ValueError("forecast evaluation deltas must be an object")
    _require_exact_fields(deltas, DELTA_FIELDS, "forecast evaluation deltas")
    if deltas["comparison"] != "chronos_minus_proxy":
        raise ValueError("forecast evaluation delta comparison is invalid")
    for field in METRIC_FIELDS:
        _finite_number(deltas[field], f"delta {field}")
        expected_delta = _round(results[0][field] - results[1][field])
        if deltas[field] != expected_delta:
            raise ValueError(f"forecast evaluation delta {field} is inconsistent")

    flags = report["safety_flags"]
    if not isinstance(flags, Mapping):
        raise ValueError("forecast evaluation safety_flags must be an object")
    _require_exact_fields(flags, SAFETY_FIELDS, "forecast evaluation safety_flags")
    expected_flags = {
        "local_only": True,
        "synthetic_only": True,
        "reports_aligned": True,
        "labels_sent_to_backend": False,
        "no_future_data": True,
        "external_services_used": False,
        "deployment_allowed": False,
    }
    if dict(flags) != expected_flags:
        raise ValueError("forecast evaluation safety_flags are invalid")
    if report["non_claims"] != NON_CLAIMS:
        raise ValueError("forecast evaluation non_claims are invalid")


def validate_replay_analytical_consistency(
    proxy_report: Mapping[str, Any],
    chronos_report: Mapping[str, Any],
    cohort_rows: Sequence[Mapping[str, Any]],
    anomaly_labels: Sequence[Mapping[str, Any]],
    evaluation: Mapping[str, Any],
) -> None:
    """Cross-check replay metrics against their exact residual rows and labels."""
    time_series_residual.validate_residual_report(proxy_report)
    time_series_residual.validate_residual_report(chronos_report)
    _validate_replay_expected_report(proxy_report, pretrained=False)
    _validate_replay_expected_report(chronos_report, pretrained=True)
    validate_forecast_evaluation(evaluation)
    if evaluation["schema_version"] != REPLAY_REPORT_SCHEMA_VERSION:
        raise ValueError("analytical consistency requires the replay evaluation schema")

    labels = [_validate_label(row) for row in anomaly_labels]
    label_keys = [_row_key(row) for row in labels]
    if (
        len(labels) != REPLAY_ANOMALY_COUNT
        or label_keys != sorted(label_keys)
        or len(label_keys) != len(set(label_keys))
    ):
        raise ValueError("analytical consistency anomaly labels are invalid")

    _, scored_rows, index_by_key = _validate_replay_cohort(cohort_rows, labels)
    proxy_rows = _rows_by_key(proxy_report)
    chronos_rows = _rows_by_key(chronos_report)
    if list(proxy_rows) != list(chronos_rows) or list(proxy_rows) != list(scored_rows):
        raise ValueError("analytical residual reports do not match frozen cohort row keys")
    if not set(label_keys) <= set(proxy_rows):
        raise ValueError("analytical labels must identify residual report rows")
    for key in proxy_rows:
        cohort_actual = scored_rows[key]["actual_value"]
        for row in (proxy_rows[key], chronos_rows[key]):
            if row["actual_value"] != cohort_actual:
                raise ValueError("analytical residual reports contain non-cohort actual values")
            residual_error = abs(
                float(row["actual_value"]) - float(row["forecast_mean"]) - float(row["residual"])
            )
            if residual_error > 0.000002:
                raise ValueError("analytical forecast and residual values are inconsistent")

    anomaly_keys = set(label_keys)
    for result, contract in zip(evaluation["regime_results"], REPLAY_REGIMES, strict=True):
        start = int(contract["start_index"])
        end = int(contract["end_index"])
        keys = [key for key in proxy_rows if start <= index_by_key[key] <= end]
        proxy_result = _backend_metrics(
            proxy_report,
            {key: proxy_rows[key] for key in keys},
            anomaly_keys & set(keys),
        )
        chronos_result = _backend_metrics(
            chronos_report,
            {key: chronos_rows[key] for key in keys},
            anomaly_keys & set(keys),
        )
        expected_results = sorted(
            [proxy_result, chronos_result], key=lambda item: item["backend_id"]
        )
        if result["backend_results"] != expected_results:
            raise ValueError(f"analytical regime {contract['regime_id']} metrics are inconsistent")
        if result["deltas"] != _metric_deltas(proxy_result, chronos_result):
            raise ValueError(f"analytical regime {contract['regime_id']} deltas are inconsistent")

    proxy_aggregate = _backend_metrics(proxy_report, proxy_rows, anomaly_keys)
    chronos_aggregate = _backend_metrics(chronos_report, chronos_rows, anomaly_keys)
    expected_aggregate = sorted(
        [proxy_aggregate, chronos_aggregate], key=lambda item: item["backend_id"]
    )
    aggregate = evaluation["aggregate"]
    if aggregate["backend_results"] != expected_aggregate:
        raise ValueError("analytical aggregate metrics are inconsistent")
    if aggregate["deltas"] != _metric_deltas(proxy_aggregate, chronos_aggregate):
        raise ValueError("analytical aggregate deltas are inconsistent")


def _validate_forecast_replay_evaluation(report: Mapping[str, Any]) -> None:
    _require_exact_fields(report, REPLAY_REPORT_FIELDS, "replay evaluation")
    if report["evaluation_scope"] != REPLAY_EVALUATION_SCOPE:
        raise ValueError("replay evaluation scope is invalid")
    expected_dataset = {
        "cohort_id": REPLAY_COHORT_ID,
        "cohort_sha256": REPLAY_COHORT_SHA256,
        "series_count": len(REPLAY_SERIES),
        "observations_per_series": REPLAY_OBSERVATIONS_PER_SERIES,
        "history_window": REPLAY_HISTORY_WINDOW,
        "calibration_window": REPLAY_CALIBRATION_WINDOW,
        "cadence_seconds": REPLAY_CADENCE_SECONDS,
        "scored_observation_count": REPLAY_SCORED_COUNT,
        "anomaly_label_count": REPLAY_ANOMALY_COUNT,
        "regime_count": len(REPLAY_REGIMES),
        "stress_case_count": len(REPLAY_STRESS_CASES),
        "labels_sent_to_backend": False,
    }
    dataset = report["dataset"]
    if not isinstance(dataset, Mapping):
        raise ValueError("replay evaluation dataset must be an object")
    _require_exact_fields(dataset, REPLAY_DATASET_FIELDS, "replay evaluation dataset")
    if dict(dataset) != expected_dataset:
        raise ValueError("replay evaluation dataset must match the frozen replay cohort")

    regimes = report["regime_results"]
    if not isinstance(regimes, list) or len(regimes) != len(REPLAY_REGIMES):
        raise ValueError("replay evaluation requires six regime results")
    for result, contract in zip(regimes, REPLAY_REGIMES, strict=True):
        if not isinstance(result, Mapping):
            raise ValueError("replay regime results must be objects")
        _require_exact_fields(result, REPLAY_REGIME_RESULT_FIELDS, "replay regime result")
        expected_labels = 20 if contract["regime_id"] == "anomaly_density_change" else 8
        expected_scalars = {
            "regime_id": contract["regime_id"],
            "input_sha256": contract["input_sha256"],
            "start_index": contract["start_index"],
            "end_index": contract["end_index"],
            "scored_observation_count": 80,
            "anomaly_label_count": expected_labels,
            "anomaly_density": contract["anomaly_density"],
        }
        if {key: result[key] for key in expected_scalars} != expected_scalars:
            raise ValueError(f"replay regime {contract['regime_id']} contract drifted")
        _validate_backend_results(result["backend_results"], expected_count=80)
        _validate_metric_deltas(result["deltas"], result["backend_results"])

    stresses = report["stress_results"]
    if not isinstance(stresses, list) or len(stresses) != len(REPLAY_STRESS_CASES):
        raise ValueError("replay evaluation requires two stress results")
    for result, contract in zip(stresses, REPLAY_STRESS_CASES, strict=True):
        if not isinstance(result, Mapping):
            raise ValueError("replay stress results must be objects")
        _require_exact_fields(result, REPLAY_STRESS_RESULT_FIELDS, "replay stress result")
        expected = {
            "stress_case_id": contract["stress_case_id"],
            "input_sha256": contract["input_sha256"],
            "observation_count": contract["observation_count"],
            "input_label_count": REPLAY_ANOMALY_COUNT,
            "scoring_attempted": False,
            "inference_call_count": 0,
            "expected_rejection": contract["expected_rejection"],
            "observed_rejection": contract["expected_rejection"],
        }
        if dict(result) != expected:
            raise ValueError(f"replay stress {contract['stress_case_id']} result drifted")

    aggregate = report["aggregate"]
    if not isinstance(aggregate, Mapping):
        raise ValueError("replay aggregate must be an object")
    _require_exact_fields(aggregate, REPLAY_AGGREGATE_FIELDS, "replay aggregate")
    _validate_backend_results(aggregate["backend_results"], expected_count=REPLAY_SCORED_COUNT)
    _validate_metric_deltas(aggregate["deltas"], aggregate["backend_results"])

    expected_flags = {
        "local_only": True,
        "synthetic_only": True,
        "reports_aligned": True,
        "labels_sent_to_backend": False,
        "no_future_data": True,
        "regular_grid_required": True,
        "calibration_frozen": True,
        "stress_inference_used": False,
        "external_services_used": False,
        "deployment_allowed": False,
    }
    flags = report["safety_flags"]
    if not isinstance(flags, Mapping):
        raise ValueError("replay safety_flags must be an object")
    _require_exact_fields(flags, REPLAY_SAFETY_FIELDS, "replay safety_flags")
    if dict(flags) != expected_flags:
        raise ValueError("replay safety_flags are invalid")
    if report["non_claims"] != REPLAY_NON_CLAIMS:
        raise ValueError("replay non_claims are invalid")


def dump_forecast_evaluation(
    report: Mapping[str, Any],
    path: str | Path,
    *,
    repo_root: str | Path | None = None,
) -> None:
    validate_forecast_evaluation(report)
    output = time_series_residual._validated_output_path(path, repo_root=repo_root)
    serialized = json.dumps(report, allow_nan=False, indent=2, sort_keys=True) + "\n"
    time_series_residual._atomic_write_text(output, serialized)


def _validate_replay_expected_report(report: Mapping[str, Any], *, pretrained: bool) -> None:
    expected_schema = (
        time_series_residual.PRETRAINED_REPORT_SCHEMA_VERSION
        if pretrained
        else time_series_residual.REPORT_SCHEMA_VERSION
    )
    expected_backend = CHRONOS_BACKEND_ID if pretrained else ROLLING_MEAN_PROXY_BACKEND_ID
    if report["schema_version"] != expected_schema:
        raise ValueError("replay report schema is not the expected backend contract")
    if report["forecast_backend"]["backend_id"] != expected_backend:
        raise ValueError("replay report backend identity is invalid")
    if (
        report["history_window"] != REPLAY_HISTORY_WINDOW
        or report["calibration_window"] != REPLAY_CALIBRATION_WINDOW
        or report["interval_z"] != 2.0
    ):
        raise ValueError("replay reports require fixed 64/32/2.0 settings")
    if len(report["rows"]) != REPLAY_SCORED_COUNT:
        raise ValueError("replay reports must each contain 480 scored rows")


def _validate_replay_cohort(
    cohort_rows: Sequence[Mapping[str, Any]],
    labels: Sequence[Mapping[str, Any]],
) -> tuple[list[JsonMap], dict[RowKey, JsonMap], dict[RowKey, int]]:
    if isinstance(cohort_rows, str | bytes | bytearray) or not isinstance(cohort_rows, Sequence):
        raise ValueError("replay cohort rows must be a sequence")
    normalized_with_timestamps = [
        time_series_residual._normalize_input_row(row) for row in cohort_rows
    ]
    time_series_residual._validate_input_order(normalized_with_timestamps)
    rejection = _replay_grid_rejection(normalized_with_timestamps)
    if rejection is not None:
        raise ValueError(rejection)
    normalized = [
        {
            "entity_id": row["entity_id"],
            "feature_name": row["feature_name"],
            "window_start": row["window_start"],
            "actual_value": row["actual_value"],
        }
        for row in normalized_with_timestamps
    ]
    normalized.sort(key=_row_key)
    digest = _replay_cohort_sha256(normalized, labels)
    if digest != REPLAY_COHORT_SHA256:
        raise ValueError("replay cohort, regimes, or labels do not match the pinned digest")

    by_series: dict[tuple[str, str], list[JsonMap]] = {}
    for row in normalized:
        by_series.setdefault((row["entity_id"], row["feature_name"]), []).append(row)
    scored: list[JsonMap] = []
    index_by_key: dict[RowKey, int] = {}
    for series in REPLAY_SERIES:
        rows = by_series[series]
        for index, row in enumerate(rows):
            if index >= REPLAY_SCORING_OFFSET:
                scored.append(row)
                index_by_key[_row_key(row)] = index
    scored.sort(key=_row_key)
    return normalized, {_row_key(row): row for row in scored}, index_by_key


def _replay_grid_rejection(rows: Sequence[Mapping[str, Any]]) -> str | None:
    by_series: dict[tuple[str, str], list[Mapping[str, Any]]] = {}
    for row in rows:
        by_series.setdefault((str(row["entity_id"]), str(row["feature_name"])), []).append(row)
    if set(by_series) != set(REPLAY_SERIES):
        return "missing_observation_gap"
    for series in REPLAY_SERIES:
        series_rows = sorted(by_series[series], key=lambda row: str(row["window_start"]))
        if len(series_rows) != REPLAY_OBSERVATIONS_PER_SERIES:
            return "missing_observation_gap"
        for index, row in enumerate(series_rows):
            expected = REPLAY_START + timedelta(seconds=REPLAY_CADENCE_SECONDS * index)
            actual = datetime.fromisoformat(str(row["window_start"]).replace("Z", "+00:00"))
            if actual != expected:
                return "irregular_timestamp_interval"
    return None


def _regime_descriptor(regime: Mapping[str, Any]) -> JsonMap:
    return {
        "regime_id": regime["regime_id"],
        "start_index": regime["start_index"],
        "end_index": regime["end_index"],
        "scored_observations_per_series": regime["scored_observations_per_series"],
        "anomaly_offsets": list(regime["anomaly_offsets"]),
        "anomaly_density": regime["anomaly_density"],
    }


def _replay_cohort_sha256(
    normalized_rows: Sequence[Mapping[str, Any]], labels: Sequence[Mapping[str, Any]]
) -> str:
    return _canonical_sha256(
        {
            "cohort_id": REPLAY_COHORT_ID,
            "regimes": [_regime_descriptor(regime) for regime in REPLAY_REGIMES],
            "rows": sorted((dict(row) for row in normalized_rows), key=_row_key),
            "anomaly_labels": sorted((dict(row) for row in labels), key=_row_key),
        }
    )


def _stress_sha256(
    *,
    stress_case_id: str,
    rows: Sequence[Mapping[str, Any]],
    labels: Sequence[Mapping[str, Any]],
    expected_rejection: str,
) -> str:
    return _canonical_sha256(
        {
            "stress_case_id": stress_case_id,
            "source_cohort_sha256": REPLAY_COHORT_SHA256,
            "rows": sorted((dict(row) for row in rows), key=_row_key),
            "anomaly_labels": sorted((dict(row) for row in labels), key=_row_key),
            "expected_rejection": expected_rejection,
        }
    )


def _canonical_sha256(value: Mapping[str, Any]) -> str:
    encoded = json.dumps(value, allow_nan=False, separators=(",", ":"), sort_keys=True).encode(
        "utf-8"
    )
    return hashlib.sha256(encoded).hexdigest()


def _metric_deltas(proxy_result: Mapping[str, Any], chronos_result: Mapping[str, Any]) -> JsonMap:
    return {
        "comparison": "chronos_minus_proxy",
        **{
            field: _round(float(chronos_result[field]) - float(proxy_result[field]))
            for field in METRIC_FIELDS
        },
    }


def _validate_backend_results(raw: Any, *, expected_count: int) -> None:
    if not isinstance(raw, list) or len(raw) != 2:
        raise ValueError("forecast evaluation requires exactly two backend results")
    expected_pairs = [
        (CHRONOS_BACKEND_ID, time_series_residual.PRETRAINED_REPORT_SCHEMA_VERSION),
        (ROLLING_MEAN_PROXY_BACKEND_ID, time_series_residual.REPORT_SCHEMA_VERSION),
    ]
    actual_pairs = []
    for result in raw:
        if not isinstance(result, Mapping):
            raise ValueError("forecast evaluation backend results must be objects")
        _require_exact_fields(result, RESULT_FIELDS, "forecast evaluation backend result")
        actual_pairs.append((result["backend_id"], result["report_schema"]))
        expected_point_method = (
            "median_q0_5"
            if result["backend_id"] == CHRONOS_BACKEND_ID
            else "context_arithmetic_mean"
        )
        if result["point_method"] != expected_point_method or result["count"] != expected_count:
            raise ValueError("forecast evaluation backend result contract drifted")
        for field in ("mae", "rmse", "mean_interval_width"):
            _non_negative_number(result[field], field)
        for field in (
            "interval_coverage",
            "auroc",
            "average_precision",
            "recall_at_0_75",
            "fpr_at_0_75",
        ):
            _bounded_number(result[field], field, 0.0, 1.0)
    if actual_pairs != expected_pairs:
        raise ValueError("forecast evaluation backend results must be sorted and pinned")


def _validate_metric_deltas(raw: Any, results: Any) -> None:
    if not isinstance(raw, Mapping) or not isinstance(results, list):
        raise ValueError("forecast evaluation deltas are invalid")
    _require_exact_fields(raw, DELTA_FIELDS, "forecast evaluation deltas")
    if raw["comparison"] != "chronos_minus_proxy":
        raise ValueError("forecast evaluation delta comparison is invalid")
    for field in METRIC_FIELDS:
        _finite_number(raw[field], f"delta {field}")
        if raw[field] != _round(float(results[0][field]) - float(results[1][field])):
            raise ValueError(f"forecast evaluation delta {field} is inconsistent")


def _validate_expected_report(report: Mapping[str, Any], *, pretrained: bool) -> None:
    expected_schema = (
        time_series_residual.PRETRAINED_REPORT_SCHEMA_VERSION
        if pretrained
        else time_series_residual.REPORT_SCHEMA_VERSION
    )
    expected_backend = CHRONOS_BACKEND_ID if pretrained else ROLLING_MEAN_PROXY_BACKEND_ID
    if report["schema_version"] != expected_schema:
        raise ValueError("forecast evaluation report schema is not the expected backend contract")
    if report["forecast_backend"]["backend_id"] != expected_backend:
        raise ValueError("forecast evaluation report backend identity is invalid")
    if (
        report["history_window"] != 64
        or report["calibration_window"] != 32
        or report["interval_z"] != 2.0
    ):
        raise ValueError("forecast evaluation reports require fixed 64/32/2.0 settings")
    if len(report["rows"]) != EXPECTED_SCORED_COUNT:
        raise ValueError("forecast evaluation reports must each contain 64 scored rows")


def _rows_by_key(report: Mapping[str, Any]) -> dict[RowKey, Mapping[str, Any]]:
    return {_row_key(row): row for row in report["rows"]}


def _validate_frozen_cohort(
    cohort_rows: Sequence[Mapping[str, Any]],
    labels: Sequence[Mapping[str, Any]],
) -> dict[RowKey, JsonMap]:
    if isinstance(cohort_rows, str | bytes | bytearray) or not isinstance(cohort_rows, Sequence):
        raise ValueError("forecast evaluation cohort rows must be a sequence")
    if len(cohort_rows) != EXPECTED_SERIES_COUNT * EXPECTED_OBSERVATIONS_PER_SERIES:
        raise ValueError("forecast evaluation cohort must contain exactly 256 observations")

    normalized_with_timestamps = [
        time_series_residual._normalize_input_row(row) for row in cohort_rows
    ]
    time_series_residual._validate_input_order(normalized_with_timestamps)
    normalized = [
        {
            "entity_id": row["entity_id"],
            "feature_name": row["feature_name"],
            "window_start": row["window_start"],
            "actual_value": row["actual_value"],
        }
        for row in normalized_with_timestamps
    ]
    normalized.sort(key=_row_key)

    series_rows: dict[tuple[str, str], list[JsonMap]] = {}
    for row in normalized:
        series_rows.setdefault((row["entity_id"], row["feature_name"]), []).append(row)
    if len(series_rows) != EXPECTED_SERIES_COUNT or {
        len(rows) for rows in series_rows.values()
    } != {EXPECTED_OBSERVATIONS_PER_SERIES}:
        raise ValueError("forecast evaluation cohort must contain two 128-observation series")

    digest = _cohort_sha256(normalized, labels)
    if digest != COHORT_SHA256:
        raise ValueError("forecast evaluation cohort or labels do not match the pinned digest")

    scored_rows: list[JsonMap] = []
    scoring_offset = 64 + 32
    for series_key in sorted(series_rows):
        scored_rows.extend(series_rows[series_key][scoring_offset:])
    return {_row_key(row): row for row in scored_rows}


def _cohort_sha256(
    normalized_rows: Sequence[Mapping[str, Any]],
    labels: Sequence[Mapping[str, Any]],
) -> str:
    canonical = {
        "cohort_id": COHORT_ID,
        "rows": sorted((dict(row) for row in normalized_rows), key=_row_key),
        "anomaly_labels": sorted((dict(row) for row in labels), key=_row_key),
    }
    encoded = json.dumps(
        canonical,
        allow_nan=False,
        separators=(",", ":"),
        sort_keys=True,
    ).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()


def _backend_metrics(
    report: Mapping[str, Any],
    rows: Mapping[RowKey, Mapping[str, Any]],
    anomaly_keys: set[RowKey],
) -> JsonMap:
    residuals = [float(row["residual"]) for row in rows.values()]
    widths = [float(row["forecast_upper"] - row["forecast_lower"]) for row in rows.values()]
    covered = [
        row["forecast_lower"] <= row["actual_value"] <= row["forecast_upper"]
        for row in rows.values()
    ]
    risks_and_labels = [
        (float(row["residual_risk"]), key in anomaly_keys) for key, row in rows.items()
    ]
    true_positive = sum(label and risk >= RISK_THRESHOLD for risk, label in risks_and_labels)
    false_positive = sum(not label and risk >= RISK_THRESHOLD for risk, label in risks_and_labels)
    positive_count = sum(label for _, label in risks_and_labels)
    negative_count = len(risks_and_labels) - positive_count
    settings = report["forecast_backend"]["settings"]
    point_method = settings.get("point_method", settings.get("mean_method"))
    return {
        "backend_id": report["forecast_backend"]["backend_id"],
        "report_schema": report["schema_version"],
        "point_method": point_method,
        "count": len(rows),
        "mae": _round(sum(abs(value) for value in residuals) / len(residuals)),
        "rmse": _round(math.sqrt(sum(value * value for value in residuals) / len(residuals))),
        "interval_coverage": _round(sum(covered) / len(covered)),
        "mean_interval_width": _round(sum(widths) / len(widths)),
        "auroc": _round(_auroc(risks_and_labels)),
        "average_precision": _round(_average_precision(risks_and_labels)),
        "recall_at_0_75": _round(true_positive / positive_count),
        "fpr_at_0_75": _round(false_positive / negative_count),
    }


def _auroc(scores_and_labels: Sequence[tuple[float, bool]]) -> float:
    positives = [score for score, label in scores_and_labels if label]
    negatives = [score for score, label in scores_and_labels if not label]
    wins = 0.0
    for positive in positives:
        for negative in negatives:
            wins += 1.0 if positive > negative else 0.5 if positive == negative else 0.0
    return wins / (len(positives) * len(negatives))


def _average_precision(scores_and_labels: Sequence[tuple[float, bool]]) -> float:
    groups: dict[float, list[bool]] = {}
    for score, label in scores_and_labels:
        groups.setdefault(score, []).append(label)
    positive_count = sum(label for _, label in scores_and_labels)
    true_positive = 0
    false_positive = 0
    previous_recall = 0.0
    result = 0.0
    for score in sorted(groups, reverse=True):
        labels = groups[score]
        true_positive += sum(labels)
        false_positive += len(labels) - sum(labels)
        recall = true_positive / positive_count
        precision = true_positive / (true_positive + false_positive)
        result += (recall - previous_recall) * precision
        previous_recall = recall
    return result


def _validate_label(raw: Mapping[str, Any]) -> JsonMap:
    if not isinstance(raw, Mapping):
        raise ValueError("forecast evaluation labels must be objects")
    _require_exact_fields(raw, LABEL_FIELDS, "forecast evaluation label")
    if not isinstance(raw["entity_id"], str) or not SAFE_ENTITY_ID_RE.fullmatch(raw["entity_id"]):
        raise ValueError("forecast evaluation label entity_id is unsafe")
    if not isinstance(raw["feature_name"], str) or not SAFE_FEATURE_NAME_RE.fullmatch(
        raw["feature_name"]
    ):
        raise ValueError("forecast evaluation label feature_name is unsafe")
    if not isinstance(raw["window_start"], str):
        raise ValueError("forecast evaluation label window_start must be UTC")
    try:
        timestamp = datetime.fromisoformat(raw["window_start"].replace("Z", "+00:00"))
    except ValueError as exc:
        raise ValueError("forecast evaluation label window_start must be UTC") from exc
    if timestamp.tzinfo is None or timestamp.utcoffset() != timedelta(0):
        raise ValueError("forecast evaluation label window_start must be UTC")
    if raw["is_anomaly"] is not True:
        raise ValueError("forecast evaluation label rows must identify anomalies")
    return dict(raw)


def _row_key(row: Mapping[str, Any]) -> RowKey:
    return (str(row["entity_id"]), str(row["feature_name"]), str(row["window_start"]))


def _loads_strict(text: str) -> Any:
    def reject_constant(value: str) -> None:
        raise ValueError(f"non-strict JSON constant {value}")

    def reject_duplicates(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
        result: dict[str, Any] = {}
        for key, value in pairs:
            if key in result:
                raise ValueError(f"duplicate JSON key {key!r}")
            result[key] = value
        return result

    try:
        return json.loads(
            text,
            parse_constant=reject_constant,
            object_pairs_hook=reject_duplicates,
        )
    except json.JSONDecodeError as exc:
        raise ValueError("invalid JSON") from exc


def _require_exact_fields(raw: Mapping[str, Any], expected: frozenset[str], field: str) -> None:
    actual = set(raw)
    if actual != expected:
        raise ValueError(
            f"{field} fields invalid: missing={sorted(expected - actual)}, "
            f"unknown={sorted(actual - expected)}"
        )


def _finite_number(raw: object, field: str) -> float:
    if isinstance(raw, bool) or not isinstance(raw, int | float):
        raise ValueError(f"{field} must be finite")
    value = float(raw)
    if not math.isfinite(value):
        raise ValueError(f"{field} must be finite")
    return value


def _non_negative_number(raw: object, field: str) -> float:
    value = _finite_number(raw, field)
    if value < 0.0:
        raise ValueError(f"{field} must be non-negative")
    return value


def _bounded_number(raw: object, field: str, minimum: float, maximum: float) -> float:
    value = _finite_number(raw, field)
    if not minimum <= value <= maximum:
        raise ValueError(f"{field} must be between {minimum} and {maximum}")
    return value


def _round(value: float) -> float:
    rounded = round(float(value), 12)
    return 0.0 if rounded == -0.0 else rounded


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Compare aligned offline forecast reports.")
    parser.add_argument("proxy_report", help="rolling_mean_proxy v1 residual report")
    parser.add_argument("chronos_report", help="chronos_bolt_tiny_local v2 residual report")
    parser.add_argument("windows", help="strict frozen synthetic time-window JSON/JSONL cohort")
    parser.add_argument("labels", help="sorted synthetic anomaly-only JSON/JSONL labels")
    parser.add_argument("output", help="path to atomically write evaluation JSON")
    parser.add_argument(
        "--replay",
        action="store_true",
        help="validate and compare the frozen long drift/replay cohort",
    )
    args = parser.parse_args(argv)

    generator = generate_forecast_replay_evaluation if args.replay else generate_forecast_evaluation
    label_loader = load_replay_anomaly_labels if args.replay else load_anomaly_labels
    report = generator(
        load_residual_report(args.proxy_report),
        load_residual_report(args.chronos_report),
        time_series_residual.load_time_window_rows(args.windows),
        label_loader(args.labels),
    )
    dump_forecast_evaluation(report, args.output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
