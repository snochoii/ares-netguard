"""Deterministic time-series residual anomaly evidence producer.

The v0 adapter is a stdlib-only proxy for forecast-residual evidence. It is
not a pretrained foundation model and does not download or train anything.
"""

from __future__ import annotations

import argparse
import json
import math
from collections import defaultdict
from collections.abc import Mapping, Sequence
from datetime import datetime
from pathlib import Path
from typing import Any

from ares_netguard.models.disagreement import ROW_SCHEMA_VERSION

REPORT_SCHEMA_VERSION = "time_series_residual_report.v0"
MODEL_ID = "time_series_residual"
MODEL_FAMILY = "experimental_time_series"
DEFAULT_HISTORY_WINDOW = 3
DEFAULT_INTERVAL_Z = 2.0
MIN_SCALE = 1e-6

RESIDUAL_ROW_FIELDS = (
    "entity_id",
    "feature_name",
    "window_start",
    "actual_value",
    "forecast_mean",
    "forecast_lower",
    "forecast_upper",
    "residual",
    "residual_z",
    "conformal_score",
    "residual_risk",
    "model_id",
    "model_family",
)

JsonMap = dict[str, Any]


def load_time_window_rows(path: str | Path) -> list[JsonMap]:
    """Load JSON or JSONL time-window feature rows."""
    source = Path(path)
    text = source.read_text(encoding="utf-8").strip()
    if not text:
        return []

    if source.suffix == ".jsonl":
        return [json.loads(line) for line in text.splitlines() if line.strip()]

    payload = json.loads(text)
    if isinstance(payload, list):
        return payload
    if isinstance(payload, dict) and isinstance(payload.get("rows"), list):
        return payload["rows"]
    if isinstance(payload, dict):
        return [payload]
    raise ValueError(f"unsupported time-window row payload in {source}")


def generate_residual_report(
    rows: Sequence[Mapping[str, Any]],
    *,
    history_window: int = DEFAULT_HISTORY_WINDOW,
    interval_z: float = DEFAULT_INTERVAL_Z,
) -> JsonMap:
    """Generate deterministic forecast-residual evidence from feature rows."""
    _validate_settings(history_window, interval_z)
    normalized = [_normalize_input_row(row) for row in rows]
    _validate_input_order(normalized)

    history_by_series: dict[tuple[str, str], list[float]] = defaultdict(list)
    residual_rows: list[JsonMap] = []

    for row in normalized:
        series_key = (row["entity_id"], row["feature_name"])
        history = history_by_series[series_key]
        if len(history) >= history_window:
            evidence = _build_residual_row(
                row,
                history[-history_window:],
                interval_z=interval_z,
            )
            validate_residual_evidence_row(evidence)
            residual_rows.append(evidence)
        history.append(row["actual_value"])

    residual_rows = sorted(
        residual_rows,
        key=lambda item: (item["entity_id"], item["feature_name"], item["window_start"]),
    )
    return {
        "schema_version": REPORT_SCHEMA_VERSION,
        "model_id": MODEL_ID,
        "model_family": MODEL_FAMILY,
        "history_window": history_window,
        "interval_z": _round(interval_z),
        "rows": residual_rows,
    }


def residual_evidence_to_score_rows(
    report_or_rows: Mapping[str, Any] | Sequence[Mapping[str, Any]],
) -> list[JsonMap]:
    """Convert residual evidence rows into model_score_row.v0 rows.

    Multiple residual features for the same entity/window are folded into one
    time_series_residual score using the maximum residual risk and preserving
    all feature-level evidence rows.
    """
    residual_rows = _extract_residual_rows(report_or_rows)
    groups: dict[tuple[str, str], list[Mapping[str, Any]]] = defaultdict(list)

    for row in residual_rows:
        validate_residual_evidence_row(row)
        groups[(row["entity_id"], row["window_start"])].append(row)

    score_rows: list[JsonMap] = []
    for entity_id, window_start in sorted(groups):
        evidence_rows = sorted(
            groups[(entity_id, window_start)],
            key=lambda item: (item["feature_name"], item["window_start"]),
        )
        risk = max(row["residual_risk"] for row in evidence_rows)
        score_rows.append(
            {
                "schema_version": ROW_SCHEMA_VERSION,
                "entity_id": entity_id,
                "window_start": window_start,
                "scores": {
                    MODEL_ID: {
                        "risk": _round(risk),
                        "scale": "risk",
                        "family": MODEL_FAMILY,
                        "evidence": [dict(row) for row in evidence_rows],
                    }
                },
            }
        )
    return score_rows


def validate_residual_evidence_row(row: Mapping[str, Any]) -> None:
    """Validate the strict v0 residual evidence row schema."""
    if not isinstance(row, Mapping):
        raise ValueError("residual evidence row must be an object")

    actual_fields = set(row)
    expected_fields = set(RESIDUAL_ROW_FIELDS)
    if actual_fields != expected_fields:
        missing = sorted(expected_fields - actual_fields)
        unexpected = sorted(actual_fields - expected_fields)
        details = []
        if missing:
            details.append(f"missing {missing}")
        if unexpected:
            details.append(f"unexpected {unexpected}")
        raise ValueError(f"residual evidence row fields invalid: {', '.join(details)}")

    _required_text(row, "entity_id")
    _required_text(row, "feature_name")
    _parse_window_start(_required_text(row, "window_start"))

    for field in (
        "actual_value",
        "forecast_mean",
        "forecast_lower",
        "forecast_upper",
        "residual",
        "residual_z",
        "conformal_score",
        "residual_risk",
    ):
        _finite_number(row.get(field), field)

    if row["forecast_lower"] > row["forecast_upper"]:
        raise ValueError("forecast_lower must be less than or equal to forecast_upper")
    _bounded_number(row["conformal_score"], "conformal_score", 0.0, 1.0)
    _bounded_number(row["residual_risk"], "residual_risk", 0.0, 1.0)

    if row["model_id"] != MODEL_ID:
        raise ValueError(f"model_id must be '{MODEL_ID}'")
    if row["model_family"] != MODEL_FAMILY:
        raise ValueError(f"model_family must be '{MODEL_FAMILY}'")


def dump_report(report: Mapping[str, Any], path: str | Path) -> None:
    """Write report JSON with stable formatting and strict finite numbers."""
    Path(path).write_text(
        json.dumps(report, allow_nan=False, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def _extract_residual_rows(
    report_or_rows: Mapping[str, Any] | Sequence[Mapping[str, Any]],
) -> Sequence[Mapping[str, Any]]:
    if isinstance(report_or_rows, Mapping):
        if report_or_rows.get("schema_version") != REPORT_SCHEMA_VERSION:
            raise ValueError(f"residual report requires schema_version '{REPORT_SCHEMA_VERSION}'")
        rows = report_or_rows.get("rows")
        if not isinstance(rows, list):
            raise ValueError("residual report requires a 'rows' list")
        return rows
    return report_or_rows


def _build_residual_row(
    row: Mapping[str, Any],
    history: Sequence[float],
    *,
    interval_z: float,
) -> JsonMap:
    forecast_mean = _mean(history)
    scale = max(_population_std(history, forecast_mean), MIN_SCALE)
    forecast_lower = forecast_mean - interval_z * scale
    forecast_upper = forecast_mean + interval_z * scale
    actual = row["actual_value"]
    residual = actual - forecast_mean
    residual_z = residual / scale
    conformal_score = _conformal_anomaly_score(history, forecast_mean, scale, abs(residual_z))
    z_risk = min(1.0, abs(residual_z) / 4.0)
    interval_breached = actual < forecast_lower or actual > forecast_upper
    residual_risk = max(conformal_score, z_risk, 0.75 if interval_breached else 0.0)

    return {
        "entity_id": row["entity_id"],
        "feature_name": row["feature_name"],
        "window_start": row["window_start"],
        "actual_value": _round(actual),
        "forecast_mean": _round(forecast_mean),
        "forecast_lower": _round(forecast_lower),
        "forecast_upper": _round(forecast_upper),
        "residual": _round(residual),
        "residual_z": _round(residual_z),
        "conformal_score": _round(conformal_score),
        "residual_risk": _round(residual_risk),
        "model_id": MODEL_ID,
        "model_family": MODEL_FAMILY,
    }


def _conformal_anomaly_score(
    history: Sequence[float],
    forecast_mean: float,
    scale: float,
    residual_abs_z: float,
) -> float:
    calibration_scores = [abs(value - forecast_mean) / scale for value in history]
    tail_count = sum(score >= residual_abs_z for score in calibration_scores)
    p_value = (tail_count + 1.0) / (len(calibration_scores) + 1.0)
    return 1.0 - p_value


def _normalize_input_row(row: Mapping[str, Any]) -> JsonMap:
    if not isinstance(row, Mapping):
        raise ValueError("time-window row must be an object")

    entity_id = _required_text(row, "entity_id")
    feature_name = _required_text(row, "feature_name")
    window_start = _required_text(row, "window_start")
    timestamp = _parse_window_start(window_start)
    actual_value = _finite_number(row.get("actual_value"), "actual_value")

    return {
        "entity_id": entity_id,
        "feature_name": feature_name,
        "window_start": window_start,
        "timestamp": timestamp,
        "actual_value": actual_value,
    }


def _validate_input_order(rows: Sequence[Mapping[str, Any]]) -> None:
    seen_windows: set[tuple[str, str, str]] = set()
    last_timestamp_by_series: dict[tuple[str, str], datetime] = {}

    for row in rows:
        series_key = (row["entity_id"], row["feature_name"])
        window_key = (*series_key, row["window_start"])
        if window_key in seen_windows:
            raise ValueError("duplicate window_start for entity_id/feature_name")
        seen_windows.add(window_key)

        previous = last_timestamp_by_series.get(series_key)
        if previous is not None and row["timestamp"] <= previous:
            raise ValueError(
                "time-window rows must be strictly increasing per entity_id/feature_name"
            )
        last_timestamp_by_series[series_key] = row["timestamp"]


def _validate_settings(history_window: int, interval_z: float) -> None:
    if (
        isinstance(history_window, bool)
        or not isinstance(history_window, int)
        or history_window < 1
    ):
        raise ValueError("history_window must be a positive integer")
    _bounded_number(interval_z, "interval_z", MIN_SCALE, 1000.0)


def _required_text(row: Mapping[str, Any], key: str) -> str:
    value = row.get(key)
    if not isinstance(value, str) or not value.strip():
        raise ValueError(f"time-window row requires non-empty '{key}'")
    return value


def _parse_window_start(value: str) -> datetime:
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError as exc:
        raise ValueError("window_start must be an ISO-8601 timestamp") from exc
    if parsed.tzinfo is None:
        raise ValueError("window_start must include timezone information")
    return parsed


def _finite_number(raw_value: Any, field: str) -> float:
    if isinstance(raw_value, bool) or not isinstance(raw_value, int | float):
        raise ValueError(f"{field} must be a finite number")

    value = float(raw_value)
    if not math.isfinite(value):
        raise ValueError(f"{field} must be a finite number")
    return value


def _bounded_number(raw_value: Any, field: str, lower: float, upper: float) -> float:
    value = _finite_number(raw_value, field)
    if value < lower or value > upper:
        raise ValueError(f"{field} must be between {lower} and {upper}")
    return value


def _mean(values: Sequence[float]) -> float:
    return sum(values) / len(values)


def _population_std(values: Sequence[float], mean: float) -> float:
    variance = sum((value - mean) ** 2 for value in values) / len(values)
    return math.sqrt(variance)


def _round(value: float) -> float:
    rounded = round(value, 6)
    return 0.0 if rounded == 0 else rounded


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Generate deterministic time-series residual evidence."
    )
    parser.add_argument("input", help="JSON or JSONL time-window feature rows")
    parser.add_argument("output", help="Path to write residual report JSON")
    parser.add_argument(
        "--history-window",
        type=int,
        default=DEFAULT_HISTORY_WINDOW,
        help="Number of previous rows per entity/feature used as proxy history",
    )
    parser.add_argument(
        "--interval-z",
        type=float,
        default=DEFAULT_INTERVAL_Z,
        help="Forecast interval width in population standard deviations",
    )
    args = parser.parse_args(argv)

    report = generate_residual_report(
        load_time_window_rows(args.input),
        history_window=args.history_window,
        interval_z=args.interval_z,
    )
    dump_report(report, args.output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
