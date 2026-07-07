"""Synthetic telemetry normalization and feature-window generation.

The v0 foundation accepts tiny caller-provided synthetic events only. It
normalizes safe Zeek/Suricata/Falco-like fixture rows into local telemetry
events and aggregate `feature_vector_row.v0` windows. It does not parse PCAPs,
capture traffic, discover files, enrich indicators, call external services, or
copy raw private telemetry.
"""

from __future__ import annotations

import argparse
import json
import math
import re
from collections import defaultdict
from collections.abc import Mapping, Sequence
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

INPUT_SCHEMA_VERSION = "synthetic_telemetry_event.v0"
NORMALIZED_EVENT_SCHEMA_VERSION = "telemetry_event.v0"
REPORT_SCHEMA_VERSION = "telemetry_feature_window_report.v0"
FEATURE_ROW_SCHEMA_VERSION = "feature_vector_row.v0"
SUPPORTED_WINDOW_SIZES_MINUTES = (1, 5)

SOURCE_KINDS = frozenset(
    {
        "zeek_conn",
        "zeek_dns",
        "suricata_alert",
        "host_runtime",
    }
)
INPUT_FIELDS = frozenset(
    {
        "schema_version",
        "source_kind",
        "entity_id",
        "timestamp",
        "event_count",
        "connection_count",
        "dns_query_count",
        "dns_failure_count",
        "alert_severity",
        "bytes_in",
        "bytes_out",
        "duration_ms",
        "destination_asset_id",
        "service_name",
        "tls_unknown",
        "runtime_event_count",
    }
)
REPORT_FIELDS = frozenset(
    {
        "schema_version",
        "source_event_schema",
        "normalized_event_schema",
        "feature_row_schema",
        "window_sizes_minutes",
        "row_count",
        "rows",
        "local_only",
        "synthetic_only",
        "live_capture_enabled",
        "pcap_parsing_enabled",
        "external_services_used",
        "deployment_allowed",
        "non_claims",
    }
)
FEATURE_ROW_FIELDS = frozenset({"schema_version", "entity_id", "window_start", "features"})
FEATURE_FIELDS = (
    "window_size_minutes",
    "event_count",
    "connection_count",
    "dns_query_count",
    "dns_failure_ratio",
    "alert_severity_sum",
    "max_alert_severity",
    "bytes_in_total",
    "bytes_out_total",
    "duration_ms_total",
    "destination_diversity",
    "service_diversity",
    "tls_unknown_ratio",
    "runtime_event_count",
)
NON_CLAIMS = [
    "not_live_capture",
    "not_pcap_parser",
    "not_private_telemetry",
    "not_external_enrichment",
    "not_deployment_signal",
    "not_model_training",
    "not_native_inference_execution",
]

SAFE_ENTITY_ID_RE = re.compile(r"^(?:asset|entity|fixture|host|sensor)-[a-z0-9][a-z0-9_-]{0,62}$")
SAFE_ASSET_ID_RE = re.compile(r"^(?:asset|entity|fixture|host|sensor)-[a-z0-9][a-z0-9_-]{0,62}$")
SAFE_SERVICE_RE = re.compile(r"^[a-z][a-z0-9_-]{0,40}$")
URL_RE = re.compile(r"(?i)\b(?:https?|ftp)://")
EMAIL_RE = re.compile(r"(?i)\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b")
IPV4_RE = re.compile(r"\b(?:\d{1,3}\.){3}\d{1,3}\b")
DOMAIN_RE = re.compile(
    r"(?i)\b[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?"
    r"(?:\.[a-z](?:[a-z0-9-]{0,61}[a-z0-9])?)+\b"
)
PATH_RE = re.compile(r"(?i)(?:^|[\s=])(?:/[a-z0-9._-]+){2,}|\b[a-z]:\\")
SECRET_RE = re.compile(r"(?i)\b(?:password|passwd|credential|secret|api[_-]?key|token)\b")
COMMAND_LINE_RE = re.compile(
    r"(?i)(?:^|\s)(?:bash|sh|cmd(?:\.exe)?|powershell|pwsh|curl|wget)\s"
    r"|[;&|]{2}|`|(?:^|\s)-{1,2}[a-z][\w-]*"
)
ARTIFACT_RE = re.compile(
    r"(?i)\.(?:pcapng?|parquet|joblib|pkl|onnx|pt|pth|ckpt|db|sqlite|duckdb|jsonl)\b"
)

JsonMap = dict[str, Any]


def load_synthetic_telemetry_events(path: str | Path) -> list[JsonMap]:
    """Load strict JSONL synthetic telemetry rows."""
    source = Path(path)
    if source.is_dir():
        raise ValueError(f"telemetry fixture path must be a file, not a directory: {source}")

    rows: list[JsonMap] = []
    for line_number, line in enumerate(source.read_text(encoding="utf-8").splitlines(), start=1):
        if not line.strip():
            continue
        payload = _loads_strict(line)
        if not isinstance(payload, Mapping):
            raise ValueError(f"telemetry fixture line {line_number} must be an object")
        rows.append(normalize_telemetry_event(payload, event_index=len(rows)))

    if not rows:
        raise ValueError("telemetry fixture must contain at least one event")
    return rows


def normalize_telemetry_event(row: Mapping[str, Any], *, event_index: int) -> JsonMap:
    """Validate one synthetic source row and return a normalized event."""
    _require_exact_fields(row, INPUT_FIELDS, "synthetic telemetry event")
    if row["schema_version"] != INPUT_SCHEMA_VERSION:
        raise ValueError(
            f"synthetic telemetry event requires schema_version '{INPUT_SCHEMA_VERSION}'"
        )

    source_kind = _allowed_source_kind(row["source_kind"])
    entity_id = _required_entity_id(row["entity_id"], "entity_id")
    timestamp = _required_timestamp(row["timestamp"], "timestamp")
    event_count = _non_negative_int(row["event_count"], "event_count")
    connection_count = _non_negative_int(row["connection_count"], "connection_count")
    dns_query_count = _non_negative_int(row["dns_query_count"], "dns_query_count")
    dns_failure_count = _non_negative_int(row["dns_failure_count"], "dns_failure_count")
    alert_severity = _bounded_int(row["alert_severity"], "alert_severity", 0, 5)
    bytes_in = _non_negative_number(row["bytes_in"], "bytes_in")
    bytes_out = _non_negative_number(row["bytes_out"], "bytes_out")
    duration_ms = _non_negative_number(row["duration_ms"], "duration_ms")
    destination_asset_id = _required_asset_id(row["destination_asset_id"], "destination_asset_id")
    service_name = _required_service_name(row["service_name"], "service_name")
    tls_unknown = _required_bool(row["tls_unknown"], "tls_unknown")
    runtime_event_count = _non_negative_int(row["runtime_event_count"], "runtime_event_count")

    if event_count == 0:
        raise ValueError("event_count must be greater than zero")
    if dns_failure_count > dns_query_count:
        raise ValueError("dns_failure_count cannot exceed dns_query_count")

    return {
        "schema_version": NORMALIZED_EVENT_SCHEMA_VERSION,
        "event_id": f"telemetry-event-{event_index + 1:04d}",
        "source_event_schema": INPUT_SCHEMA_VERSION,
        "source_kind": source_kind,
        "entity_id": entity_id,
        "timestamp": _format_timestamp(timestamp),
        "minute_start": _format_timestamp(_floor_window(timestamp, 1)),
        "event_count": event_count,
        "connection_count": connection_count,
        "dns_query_count": dns_query_count,
        "dns_failure_count": dns_failure_count,
        "alert_severity": alert_severity,
        "bytes_in": _round(bytes_in),
        "bytes_out": _round(bytes_out),
        "duration_ms": _round(duration_ms),
        "destination_asset_id": destination_asset_id,
        "service_name": service_name,
        "tls_unknown": tls_unknown,
        "runtime_event_count": runtime_event_count,
        "local_only": True,
        "synthetic_only": True,
    }


def generate_feature_window_report(
    events: Sequence[Mapping[str, Any]],
    *,
    window_sizes_minutes: Sequence[int] = SUPPORTED_WINDOW_SIZES_MINUTES,
) -> JsonMap:
    """Generate deterministic feature windows from normalized telemetry events."""
    normalized_events = [
        _validated_normalized_event(event, event_index=index) for index, event in enumerate(events)
    ]
    windows = _validated_window_sizes(window_sizes_minutes)

    rows: list[JsonMap] = []
    for window_size in windows:
        grouped: dict[tuple[str, str], list[JsonMap]] = defaultdict(list)
        for event in normalized_events:
            timestamp = _required_timestamp(event["timestamp"], "timestamp")
            window_start = _format_timestamp(_floor_window(timestamp, window_size))
            grouped[(event["entity_id"], window_start)].append(dict(event))

        for (entity_id, window_start), window_events in sorted(grouped.items()):
            rows.append(_feature_row(entity_id, window_start, window_size, window_events))

    report = {
        "schema_version": REPORT_SCHEMA_VERSION,
        "source_event_schema": INPUT_SCHEMA_VERSION,
        "normalized_event_schema": NORMALIZED_EVENT_SCHEMA_VERSION,
        "feature_row_schema": FEATURE_ROW_SCHEMA_VERSION,
        "window_sizes_minutes": list(windows),
        "row_count": len(rows),
        "rows": rows,
        "local_only": True,
        "synthetic_only": True,
        "live_capture_enabled": False,
        "pcap_parsing_enabled": False,
        "external_services_used": False,
        "deployment_allowed": False,
        "non_claims": list(NON_CLAIMS),
    }
    validate_feature_window_report(report)
    return report


def validate_feature_window_report(report: Mapping[str, Any]) -> None:
    """Validate the strict telemetry feature-window report contract."""
    _require_exact_fields(report, REPORT_FIELDS, "telemetry feature report")
    if report["schema_version"] != REPORT_SCHEMA_VERSION:
        raise ValueError(f"feature report requires schema_version '{REPORT_SCHEMA_VERSION}'")
    if report["source_event_schema"] != INPUT_SCHEMA_VERSION:
        raise ValueError(f"source_event_schema must be '{INPUT_SCHEMA_VERSION}'")
    if report["normalized_event_schema"] != NORMALIZED_EVENT_SCHEMA_VERSION:
        raise ValueError(f"normalized_event_schema must be '{NORMALIZED_EVENT_SCHEMA_VERSION}'")
    if report["feature_row_schema"] != FEATURE_ROW_SCHEMA_VERSION:
        raise ValueError(f"feature_row_schema must be '{FEATURE_ROW_SCHEMA_VERSION}'")
    windows = _validated_window_sizes(_bounded_list(report["window_sizes_minutes"], "windows"))
    rows = _bounded_list(report["rows"], "rows")
    if report["row_count"] != len(rows):
        raise ValueError("row_count must equal rows length")
    _validate_required_flag("local_only", report["local_only"], True)
    _validate_required_flag("synthetic_only", report["synthetic_only"], True)
    _validate_required_flag("live_capture_enabled", report["live_capture_enabled"], False)
    _validate_required_flag("pcap_parsing_enabled", report["pcap_parsing_enabled"], False)
    _validate_required_flag("external_services_used", report["external_services_used"], False)
    _validate_required_flag("deployment_allowed", report["deployment_allowed"], False)
    if report["non_claims"] != NON_CLAIMS:
        raise ValueError("non_claims drifted")

    seen: set[tuple[str, str, int]] = set()
    for row in rows:
        _validate_feature_row(row, set(windows))
        key = (
            row["entity_id"],
            row["window_start"],
            int(row["features"]["window_size_minutes"]),
        )
        if key in seen:
            raise ValueError("duplicate feature window row")
        seen.add(key)


def dump_feature_window_report(report: Mapping[str, Any], path: str | Path) -> None:
    """Write a feature-window report to an approved local output path."""
    validate_feature_window_report(report)
    _validate_output_path(path)
    Path(path).write_text(
        json.dumps(report, allow_nan=False, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def _feature_row(
    entity_id: str,
    window_start: str,
    window_size_minutes: int,
    events: Sequence[Mapping[str, Any]],
) -> JsonMap:
    event_count = sum(int(event["event_count"]) for event in events)
    dns_query_count = sum(int(event["dns_query_count"]) for event in events)
    dns_failure_count = sum(int(event["dns_failure_count"]) for event in events)
    tls_unknown_events = sum(int(event["event_count"]) for event in events if event["tls_unknown"])
    features = {
        "window_size_minutes": window_size_minutes,
        "event_count": float(event_count),
        "connection_count": float(sum(int(event["connection_count"]) for event in events)),
        "dns_query_count": float(dns_query_count),
        "dns_failure_ratio": _round(dns_failure_count / dns_query_count)
        if dns_query_count
        else 0.0,
        "alert_severity_sum": float(sum(int(event["alert_severity"]) for event in events)),
        "max_alert_severity": float(max(int(event["alert_severity"]) for event in events)),
        "bytes_in_total": _round(sum(float(event["bytes_in"]) for event in events)),
        "bytes_out_total": _round(sum(float(event["bytes_out"]) for event in events)),
        "duration_ms_total": _round(sum(float(event["duration_ms"]) for event in events)),
        "destination_diversity": float(
            len({str(event["destination_asset_id"]) for event in events})
        ),
        "service_diversity": float(len({str(event["service_name"]) for event in events})),
        "tls_unknown_ratio": _round(tls_unknown_events / event_count) if event_count else 0.0,
        "runtime_event_count": float(sum(int(event["runtime_event_count"]) for event in events)),
    }
    return {
        "schema_version": FEATURE_ROW_SCHEMA_VERSION,
        "entity_id": entity_id,
        "window_start": window_start,
        "features": features,
    }


def _validated_normalized_event(event: Mapping[str, Any], *, event_index: int) -> JsonMap:
    if event.get("schema_version") == INPUT_SCHEMA_VERSION:
        return normalize_telemetry_event(event, event_index=event_index)
    if event.get("schema_version") != NORMALIZED_EVENT_SCHEMA_VERSION:
        raise ValueError("event must be a synthetic source row or normalized telemetry event")
    required = {
        "schema_version",
        "event_id",
        "source_event_schema",
        "source_kind",
        "entity_id",
        "timestamp",
        "minute_start",
        "event_count",
        "connection_count",
        "dns_query_count",
        "dns_failure_count",
        "alert_severity",
        "bytes_in",
        "bytes_out",
        "duration_ms",
        "destination_asset_id",
        "service_name",
        "tls_unknown",
        "runtime_event_count",
        "local_only",
        "synthetic_only",
    }
    _require_exact_fields(event, frozenset(required), "normalized telemetry event")
    _validate_required_flag("local_only", event["local_only"], True)
    _validate_required_flag("synthetic_only", event["synthetic_only"], True)
    if event["event_id"] != f"telemetry-event-{event_index + 1:04d}":
        raise ValueError("event_id sequence drifted")
    if event["source_event_schema"] != INPUT_SCHEMA_VERSION:
        raise ValueError(f"source_event_schema must be '{INPUT_SCHEMA_VERSION}'")
    _allowed_source_kind(event["source_kind"])
    _required_entity_id(event["entity_id"], "entity_id")
    timestamp = _required_timestamp(event["timestamp"], "timestamp")
    minute_start = _format_timestamp(_floor_window(timestamp, 1))
    if event["minute_start"] != minute_start:
        raise ValueError("minute_start must match timestamp minute")
    event_count = _non_negative_int(event["event_count"], "event_count")
    _non_negative_int(event["connection_count"], "connection_count")
    dns_query_count = _non_negative_int(event["dns_query_count"], "dns_query_count")
    dns_failure_count = _non_negative_int(event["dns_failure_count"], "dns_failure_count")
    if event_count == 0:
        raise ValueError("event_count must be greater than zero")
    if dns_failure_count > dns_query_count:
        raise ValueError("dns_failure_count cannot exceed dns_query_count")
    _bounded_int(event["alert_severity"], "alert_severity", 0, 5)
    _non_negative_number(event["bytes_in"], "bytes_in")
    _non_negative_number(event["bytes_out"], "bytes_out")
    _non_negative_number(event["duration_ms"], "duration_ms")
    _required_asset_id(event["destination_asset_id"], "destination_asset_id")
    _required_service_name(event["service_name"], "service_name")
    _required_bool(event["tls_unknown"], "tls_unknown")
    _non_negative_int(event["runtime_event_count"], "runtime_event_count")
    return dict(event)


def _validate_feature_row(row: Any, allowed_windows: set[int]) -> None:
    if not isinstance(row, Mapping):
        raise ValueError("feature row must be an object")
    _require_exact_fields(row, FEATURE_ROW_FIELDS, "feature row")
    if row["schema_version"] != FEATURE_ROW_SCHEMA_VERSION:
        raise ValueError(f"feature row requires schema_version '{FEATURE_ROW_SCHEMA_VERSION}'")
    _required_entity_id(row["entity_id"], "entity_id")
    _required_timestamp(row["window_start"], "window_start")
    features = row["features"]
    if not isinstance(features, Mapping):
        raise ValueError("features must be an object")
    if tuple(sorted(features)) != tuple(sorted(FEATURE_FIELDS)):
        raise ValueError("feature keys drifted")
    window_size = _bounded_int(features["window_size_minutes"], "window_size_minutes", 1, 5)
    if window_size not in allowed_windows:
        raise ValueError("feature row window_size_minutes is not declared")
    for name in FEATURE_FIELDS:
        value = _non_negative_number(features[name], name)
        if name.endswith("_ratio") and value > 1.0:
            raise ValueError(f"{name} must be between 0 and 1")


def _validated_window_sizes(raw_windows: Sequence[Any]) -> tuple[int, ...]:
    if not raw_windows:
        raise ValueError("at least one window size is required")
    windows = tuple(
        sorted(_bounded_int(value, "window_size_minutes", 1, 5) for value in raw_windows)
    )
    if len(set(windows)) != len(windows):
        raise ValueError("window sizes must be unique")
    if any(window not in SUPPORTED_WINDOW_SIZES_MINUTES for window in windows):
        raise ValueError("unsupported window size")
    return windows


def _validate_output_path(path: str | Path) -> None:
    output = Path(path)
    if output.is_dir():
        raise ValueError("output path must be a file")
    repo_root = Path.cwd().resolve()
    resolved = output.resolve(strict=False)
    try:
        relative = resolved.relative_to(repo_root)
    except ValueError:
        return
    allowed_prefixes = (
        Path("data/features"),
        Path("data/reports"),
        Path(".runtime"),
        Path("artifacts"),
    )
    if not any(relative == prefix or prefix in relative.parents for prefix in allowed_prefixes):
        raise ValueError("repository output paths must be under ignored runtime roots")


def _loads_strict(text: str) -> Any:
    def reject_constant(value: str) -> None:
        raise ValueError(f"non-strict JSON constant '{value}' is not allowed")

    return json.loads(text, parse_constant=reject_constant)


def _require_exact_fields(row: Mapping[str, Any], expected: frozenset[str], label: str) -> None:
    actual = set(row)
    if actual != expected:
        missing = sorted(expected - actual)
        unexpected = sorted(actual - expected)
        details = []
        if missing:
            details.append(f"missing {missing}")
        if unexpected:
            details.append(f"unexpected {unexpected}")
        raise ValueError(f"{label} fields invalid: {', '.join(details)}")


def _allowed_source_kind(value: Any) -> str:
    if not isinstance(value, str) or value not in SOURCE_KINDS:
        raise ValueError("source_kind is unsupported")
    return value


def _required_entity_id(value: Any, field: str) -> str:
    text = _required_safe_text(value, field)
    if not SAFE_ENTITY_ID_RE.fullmatch(text):
        raise ValueError(f"{field} must be a synthetic/coarse entity identifier")
    return text


def _required_asset_id(value: Any, field: str) -> str:
    text = _required_safe_text(value, field)
    if not SAFE_ASSET_ID_RE.fullmatch(text):
        raise ValueError(f"{field} must be a synthetic/coarse asset identifier")
    return text


def _required_service_name(value: Any, field: str) -> str:
    text = _required_safe_text(value, field)
    if not SAFE_SERVICE_RE.fullmatch(text):
        raise ValueError(f"{field} must be a sanitized service label")
    return text


def _required_safe_text(value: Any, field: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise ValueError(f"{field} must be a non-empty string")
    text = value.strip()
    if len(text) > 96:
        raise ValueError(f"{field} exceeds maximum string length")
    lowered = text.lower()
    if (
        URL_RE.search(text)
        or EMAIL_RE.search(text)
        or IPV4_RE.search(text)
        or DOMAIN_RE.search(text)
        or PATH_RE.search(text)
        or SECRET_RE.search(text)
        or COMMAND_LINE_RE.search(text)
        or ARTIFACT_RE.search(text)
        or any(part in lowered for part in ("password", "passwd", "credential", "secret", "token"))
    ):
        raise ValueError(f"{field} contains unsafe raw identifier or artifact content")
    return text


def _required_timestamp(value: Any, field: str) -> datetime:
    if not isinstance(value, str):
        raise ValueError(f"{field} must be an ISO-8601 timestamp")
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError as exc:
        raise ValueError(f"{field} must be an ISO-8601 timestamp") from exc
    if parsed.tzinfo is None:
        raise ValueError(f"{field} must include timezone information")
    return parsed.astimezone(UTC)


def _bounded_list(value: Any, field: str) -> list[Any]:
    if not isinstance(value, list):
        raise ValueError(f"{field} must be a list")
    if len(value) > 1000:
        raise ValueError(f"{field} has too many entries")
    return value


def _required_bool(value: Any, field: str) -> bool:
    if not isinstance(value, bool):
        raise ValueError(f"{field} must be a boolean")
    return value


def _non_negative_int(value: Any, field: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int):
        raise ValueError(f"{field} must be a non-negative integer")
    if value < 0:
        raise ValueError(f"{field} must be a non-negative integer")
    return value


def _bounded_int(value: Any, field: str, lower: int, upper: int) -> int:
    parsed = _non_negative_int(value, field)
    if parsed < lower or parsed > upper:
        raise ValueError(f"{field} must be between {lower} and {upper}")
    return parsed


def _non_negative_number(value: Any, field: str) -> float:
    if isinstance(value, bool) or not isinstance(value, int | float):
        raise ValueError(f"{field} must be a non-negative finite number")
    parsed = float(value)
    if not math.isfinite(parsed) or parsed < 0:
        raise ValueError(f"{field} must be a non-negative finite number")
    return parsed


def _validate_required_flag(field: str, actual: Any, expected: bool) -> None:
    if actual is not expected:
        raise ValueError(f"{field} must be {expected}")


def _floor_window(timestamp: datetime, window_size_minutes: int) -> datetime:
    minute_bucket = (timestamp.minute // window_size_minutes) * window_size_minutes
    return timestamp.replace(minute=minute_bucket, second=0, microsecond=0)


def _format_timestamp(timestamp: datetime) -> str:
    normalized = timestamp.astimezone(UTC).replace(microsecond=0)
    return normalized.isoformat().replace("+00:00", "Z")


def _round(value: float) -> float:
    rounded = round(value, 6)
    return 0.0 if rounded == 0 else rounded


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Generate synthetic telemetry feature-window rows."
    )
    parser.add_argument("telemetry_events", help="synthetic_telemetry_event.v0 JSONL fixture")
    parser.add_argument("output", help="Path to write telemetry_feature_window_report.v0 JSON")
    args = parser.parse_args(argv)

    events = load_synthetic_telemetry_events(args.telemetry_events)
    dump_feature_window_report(generate_feature_window_report(events), args.output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
