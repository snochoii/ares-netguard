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


def validate_forecast_evaluation(report: Mapping[str, Any]) -> None:
    if not isinstance(report, Mapping):
        raise ValueError("forecast evaluation must be an object")
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
    args = parser.parse_args(argv)

    report = generate_forecast_evaluation(
        load_residual_report(args.proxy_report),
        load_residual_report(args.chronos_report),
        time_series_residual.load_time_window_rows(args.windows),
        load_anomaly_labels(args.labels),
    )
    dump_forecast_evaluation(report, args.output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
