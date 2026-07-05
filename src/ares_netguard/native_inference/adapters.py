"""Strict stdlib native-inference adapter reference path.

The v0 adapter proves promotion metadata, feature ordering, and score-row
compatibility. It does not load ONNX, pickle, database, PCAP, or other runtime
artifacts and it does not perform capture, probing, enrichment, or training.
"""

from __future__ import annotations

import argparse
import ipaddress
import json
import math
import re
from collections.abc import Mapping, Sequence
from datetime import datetime
from pathlib import Path
from typing import Any

from ares_netguard.models.disagreement import ROW_SCHEMA_VERSION

MANIFEST_SCHEMA_VERSION = "native_inference_manifest.v0"
FEATURE_ROW_SCHEMA_VERSION = "feature_vector_row.v0"
SUPPORTED_EXPORT_FORMAT = "stdlib_linear_score.v0"
SUPPORTED_INFERENCE_RUNTIME = "stdlib_reference"
SUPPORTED_ADAPTER_KIND = "linear_score.v0"
SUPPORTED_NORMALIZATION = "logistic"

MAX_STRING_LENGTH = 1024
MAX_LIST_LENGTH = 10000
MAX_MAPPING_LENGTH = 128
MAX_DEPTH = 16

MANIFEST_FIELDS = frozenset(
    {
        "schema_version",
        "model_id",
        "model_family",
        "feature_schema_version",
        "feature_columns",
        "training_data_summary",
        "evaluation_summary",
        "calibration_summary",
        "export_format",
        "inference_runtime",
        "privacy_safety_notes",
        "adapter",
    }
)
ADAPTER_FIELDS = frozenset({"kind", "weights", "bias", "normalization"})
FEATURE_ROW_FIELDS = frozenset({"schema_version", "entity_id", "window_start", "features"})

JsonMap = dict[str, Any]

SAFE_ENTITY_ID_RE = re.compile(r"^(?:asset|entity|fixture|host|sensor)-[a-z0-9][a-z0-9_-]{0,62}$")
SAFE_MODEL_ID_RE = re.compile(r"^[a-z][a-z0-9_-]{0,80}$")
SAFE_MODEL_FAMILY_RE = re.compile(r"^[a-z][a-z0-9_]{0,80}$")
SAFE_FEATURE_NAME_RE = re.compile(r"^[a-z][a-z0-9_]{0,63}$")
URL_RE = re.compile(r"(?i)\b(?:https?|ftp)://")
EMAIL_RE = re.compile(r"(?i)\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b")
IPV4_RE = re.compile(r"\b(?:\d{1,3}\.){3}\d{1,3}\b")
DOMAIN_RE = re.compile(
    r"(?i)\b[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?"
    r"(?:\.[a-z](?:[a-z0-9-]{0,61}[a-z0-9])?)+\b"
)
PATH_RE = re.compile(r"(?i)(?:^|[\s=])(?:/[a-z0-9._-]+){2,}|\b[a-z]:\\")
SECRET_RE = re.compile(r"(?i)\b(?:password|passwd|credential|secret|api[_-]?key)\b")
COMMAND_LINE_RE = re.compile(
    r"(?i)(?:^|\s)(?:bash|sh|cmd(?:\.exe)?|powershell|pwsh|curl|wget)\s"
    r"|[;&|]{2}|`|(?:^|\s)-{1,2}[a-z][\w-]*"
)
ARTIFACT_EXT_RE = re.compile(
    r"(?i)\.(?:pcapng?|parquet|joblib|pkl|onnx|pt|pth|ckpt|db|sqlite|duckdb|jsonl)\b"
)
FORBIDDEN_KEY_PARTS = (
    "payload",
    "pcap",
    "credential",
    "password",
    "passwd",
    "secret",
    "api_key",
    "apikey",
    "private_key",
    "command",
    "cmdline",
    "cmd_line",
    "path",
    "artifact",
)


def load_manifest(path: str | Path) -> JsonMap:
    """Load and validate a native_inference_manifest.v0 JSON file."""
    source = _input_file(path, "manifest")
    payload = _loads_strict(source.read_text(encoding="utf-8"))
    if not isinstance(payload, Mapping):
        raise ValueError(f"manifest payload must be an object: {source}")

    manifest = dict(payload)
    validate_manifest(manifest)
    return manifest


def load_feature_rows(path: str | Path) -> list[JsonMap]:
    """Load JSON or JSONL feature_vector_row.v0 rows using strict JSON constants."""
    source = _input_file(path, "feature rows")
    text = source.read_text(encoding="utf-8").strip()
    if not text:
        return []

    if source.suffix == ".jsonl":
        rows = [_loads_strict(line) for line in text.splitlines() if line.strip()]
    else:
        payload = _loads_strict(text)
        if isinstance(payload, list):
            rows = payload
        elif isinstance(payload, Mapping) and isinstance(payload.get("rows"), list):
            rows = payload["rows"]
        elif isinstance(payload, Mapping):
            rows = [payload]
        else:
            raise ValueError(f"unsupported feature row payload in {source}")

    loaded: list[JsonMap] = []
    for row in rows:
        if not isinstance(row, Mapping):
            raise ValueError("feature row must be an object")
        loaded.append(dict(row))
    return loaded


def validate_manifest(manifest: Mapping[str, Any]) -> None:
    """Validate the strict v0 native inference promotion manifest."""
    _manifest_spec(manifest)


def validate_feature_row(
    row: Mapping[str, Any],
    feature_columns: Sequence[str],
) -> None:
    """Validate one feature_vector_row.v0 against the manifest feature order."""
    _validated_feature_row(row, feature_columns)


def score_feature_rows(
    manifest: Mapping[str, Any],
    rows: Sequence[Mapping[str, Any]],
) -> list[JsonMap]:
    """Score feature rows and emit model_score_row.v0 rows."""
    spec = _manifest_spec(manifest)
    score_rows: list[JsonMap] = []
    seen_windows: set[tuple[str, str]] = set()

    for raw_row in rows:
        row = _validated_feature_row(raw_row, spec["feature_columns"])
        window_key = (row["entity_id"], row["window_start"])
        if window_key in seen_windows:
            raise ValueError("duplicate feature row for entity_id/window_start")
        seen_windows.add(window_key)

        linear_score, risk, contributions = _linear_score(row["features"], spec)
        score_rows.append(
            {
                "schema_version": ROW_SCHEMA_VERSION,
                "entity_id": row["entity_id"],
                "window_start": row["window_start"],
                "scores": {
                    spec["model_id"]: {
                        "risk": risk,
                        "scale": "risk",
                        "family": spec["model_family"],
                        "evidence": [
                            {
                                "adapter_kind": SUPPORTED_ADAPTER_KIND,
                                "feature_schema_version": FEATURE_ROW_SCHEMA_VERSION,
                                "feature_columns": list(spec["feature_columns"]),
                                "linear_score": _round(linear_score),
                                "normalization": SUPPORTED_NORMALIZATION,
                                "feature_contributions": contributions,
                            }
                        ],
                    }
                },
            }
        )

    return sorted(score_rows, key=lambda item: (item["entity_id"], item["window_start"]))


def dump_score_rows(rows: Sequence[Mapping[str, Any]], path: str | Path) -> None:
    """Write model_score_row.v0 rows as strict JSON."""
    output = Path(path)
    if output.is_dir():
        raise ValueError(f"output path must be a file, not a directory: {output}")
    output.write_text(
        json.dumps(list(rows), allow_nan=False, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def _manifest_spec(manifest: Mapping[str, Any]) -> JsonMap:
    if not isinstance(manifest, Mapping):
        raise ValueError("native inference manifest must be an object")

    _require_exact_fields(manifest, MANIFEST_FIELDS, "native inference manifest")
    _validate_safe_tree(manifest, "native inference manifest")

    if manifest["schema_version"] != MANIFEST_SCHEMA_VERSION:
        raise ValueError(f"unknown manifest schema_version '{manifest['schema_version']}'")
    if manifest["feature_schema_version"] != FEATURE_ROW_SCHEMA_VERSION:
        raise ValueError(f"feature_schema_version must be '{FEATURE_ROW_SCHEMA_VERSION}'")
    if manifest["export_format"] != SUPPORTED_EXPORT_FORMAT:
        raise ValueError(f"unsupported export_format '{manifest['export_format']}'")
    if manifest["inference_runtime"] != SUPPORTED_INFERENCE_RUNTIME:
        raise ValueError(f"unsupported inference_runtime '{manifest['inference_runtime']}'")

    model_id = _required_pattern(
        manifest["model_id"], "model_id", SAFE_MODEL_ID_RE, "sanitized model identifier"
    )
    model_family = _required_pattern(
        manifest["model_family"],
        "model_family",
        SAFE_MODEL_FAMILY_RE,
        "sanitized model family",
    )
    feature_columns = _feature_columns(manifest["feature_columns"])
    _summary_mapping(manifest["training_data_summary"], "training_data_summary")
    _summary_mapping(manifest["evaluation_summary"], "evaluation_summary")
    _summary_mapping(manifest["calibration_summary"], "calibration_summary")
    _privacy_notes(manifest["privacy_safety_notes"])
    adapter = _adapter_spec(manifest["adapter"], feature_columns)

    return {
        "model_id": model_id,
        "model_family": model_family,
        "feature_columns": feature_columns,
        "weights": adapter["weights"],
        "bias": adapter["bias"],
    }


def _adapter_spec(adapter: Any, feature_columns: Sequence[str]) -> JsonMap:
    if not isinstance(adapter, Mapping):
        raise ValueError("adapter must be an object")
    _require_exact_fields(adapter, ADAPTER_FIELDS, "adapter")
    if adapter["kind"] != SUPPORTED_ADAPTER_KIND:
        raise ValueError(f"unsupported adapter kind '{adapter['kind']}'")
    if adapter["normalization"] != SUPPORTED_NORMALIZATION:
        raise ValueError(f"unsupported adapter normalization '{adapter['normalization']}'")

    raw_weights = _bounded_list(adapter["weights"], "adapter.weights")
    if len(raw_weights) != len(feature_columns):
        raise ValueError("adapter.weights length must match feature_columns length")
    weights = [
        _finite_number(weight, f"adapter.weights[{index}]")
        for index, weight in enumerate(raw_weights)
    ]
    bias = _finite_number(adapter["bias"], "adapter.bias")
    return {"weights": weights, "bias": bias}


def _validated_feature_row(row: Mapping[str, Any], feature_columns: Sequence[str]) -> JsonMap:
    if not isinstance(row, Mapping):
        raise ValueError("feature row must be an object")

    _require_exact_fields(row, FEATURE_ROW_FIELDS, "feature row")
    _validate_safe_tree(row, "feature row")
    if row["schema_version"] != FEATURE_ROW_SCHEMA_VERSION:
        raise ValueError(f"unknown feature row schema_version '{row['schema_version']}'")

    entity_id = _required_pattern(
        row["entity_id"],
        "entity_id",
        SAFE_ENTITY_ID_RE,
        "synthetic/coarse entity identifier",
    )
    window_start = _required_window_start(row["window_start"], "window_start")
    features = _feature_values(row["features"], feature_columns)
    return {
        "entity_id": entity_id,
        "window_start": window_start,
        "features": features,
    }


def _feature_values(raw_features: Any, feature_columns: Sequence[str]) -> dict[str, float]:
    if not isinstance(raw_features, Mapping):
        raise ValueError("features must be an object")

    actual = set(raw_features)
    expected = set(feature_columns)
    if actual != expected:
        missing = sorted(expected - actual)
        unexpected = sorted(actual - expected)
        details = []
        if missing:
            details.append(f"missing {missing}")
        if unexpected:
            details.append(f"unexpected {unexpected}")
        raise ValueError(f"features fields invalid: {', '.join(details)}")

    return {
        column: _finite_number(raw_features[column], f"features.{column}")
        for column in feature_columns
    }


def _feature_columns(raw_columns: Any) -> list[str]:
    columns = _bounded_list(raw_columns, "feature_columns")
    if not columns:
        raise ValueError("feature_columns must not be empty")

    normalized: list[str] = []
    seen: set[str] = set()
    for index, raw_column in enumerate(columns):
        column = _required_pattern(
            raw_column,
            f"feature_columns[{index}]",
            SAFE_FEATURE_NAME_RE,
            "snake_case feature name",
        )
        _validate_key(column, "feature_columns")
        if column in seen:
            raise ValueError("feature_columns must not contain duplicates")
        seen.add(column)
        normalized.append(column)
    return normalized


def _summary_mapping(value: Any, field: str) -> Mapping[str, Any]:
    if not isinstance(value, Mapping) or not value:
        raise ValueError(f"{field} must be a non-empty object")
    return value


def _privacy_notes(value: Any) -> list[str]:
    notes = _bounded_list(value, "privacy_safety_notes")
    if not notes:
        raise ValueError("privacy_safety_notes must not be empty")
    return [
        _required_text(note, f"privacy_safety_notes[{index}]") for index, note in enumerate(notes)
    ]


def _linear_score(
    features: Mapping[str, float],
    spec: Mapping[str, Any],
) -> tuple[float, float, list[JsonMap]]:
    linear_score = float(spec["bias"])
    contributions: list[JsonMap] = []

    for column, weight in zip(spec["feature_columns"], spec["weights"], strict=True):
        value = features[column]
        contribution = value * weight
        linear_score += contribution
        contributions.append(
            {
                "feature_name": column,
                "feature_value": _round(value),
                "weight": _round(weight),
                "contribution": _round(contribution),
            }
        )

    return linear_score, _round(_logistic(linear_score)), contributions


def _input_file(path: str | Path, label: str) -> Path:
    source = Path(path)
    if source.is_dir():
        raise ValueError(f"{label} path must be a file, not a directory: {source}")
    return source


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


def _loads_strict(text: str) -> Any:
    def reject_constant(value: str) -> None:
        raise ValueError(f"non-strict JSON constant '{value}' is not allowed")

    return json.loads(text, parse_constant=reject_constant)


def _validate_safe_tree(value: Any, label: str, *, depth: int = 0) -> None:
    if depth > MAX_DEPTH:
        raise ValueError(f"{label} exceeds maximum nesting depth")

    if isinstance(value, Mapping):
        if len(value) > MAX_MAPPING_LENGTH:
            raise ValueError(f"{label} has too many object fields")
        for key, item in value.items():
            if not isinstance(key, str):
                raise ValueError(f"{label} object keys must be strings")
            _validate_key(key, label)
            _validate_safe_tree(item, f"{label}.{key}", depth=depth + 1)
        return

    if isinstance(value, list):
        if len(value) > MAX_LIST_LENGTH:
            raise ValueError(f"{label} has too many list entries")
        for index, item in enumerate(value):
            _validate_safe_tree(item, f"{label}[{index}]", depth=depth + 1)
        return

    if isinstance(value, str):
        _required_text(value, label)
        return

    if isinstance(value, bool) or value is None:
        return

    if isinstance(value, int | float):
        if not math.isfinite(float(value)):
            raise ValueError(f"{label} must contain only finite numbers")
        return

    raise ValueError(f"{label} contains unsupported value type")


def _validate_key(key: str, label: str) -> None:
    if not key or len(key) > MAX_STRING_LENGTH:
        raise ValueError(f"{label} contains invalid object key length")
    lowered = key.lower()
    if any(part in lowered for part in FORBIDDEN_KEY_PARTS):
        raise ValueError(f"{label} contains forbidden raw field '{key}'")
    _reject_unsafe_text(key, f"{label} object key")


def _required_pattern(value: Any, field: str, pattern: re.Pattern[str], description: str) -> str:
    text = _required_text(value, field)
    if not pattern.fullmatch(text):
        raise ValueError(f"{field} must be a {description}")
    return text


def _required_window_start(value: Any, field: str) -> str:
    text = _required_text(value, field)
    try:
        parsed = datetime.fromisoformat(text.replace("Z", "+00:00"))
    except ValueError as exc:
        raise ValueError(f"{field} must be an ISO-8601 timestamp") from exc
    if parsed.tzinfo is None:
        raise ValueError(f"{field} must include timezone information")
    return text


def _required_text(value: Any, field: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise ValueError(f"{field} must be a non-empty string")
    text = value.strip()
    if len(text) > MAX_STRING_LENGTH:
        raise ValueError(f"{field} exceeds maximum string length")
    _reject_unsafe_text(text, field)
    return text


def _reject_unsafe_text(value: str, field: str) -> None:
    if (
        URL_RE.search(value)
        or EMAIL_RE.search(value)
        or IPV4_RE.search(value)
        or DOMAIN_RE.search(value)
        or PATH_RE.search(value)
        or SECRET_RE.search(value)
        or COMMAND_LINE_RE.search(value)
        or ARTIFACT_EXT_RE.search(value)
        or _contains_ip_literal(value)
    ):
        raise ValueError(f"{field} contains unsafe raw identifier content")


def _contains_ip_literal(value: str) -> bool:
    for candidate in re.split(r"[\s,;|/]+", value):
        for cleaned in _ip_literal_candidates(candidate):
            try:
                ipaddress.ip_address(cleaned)
            except ValueError:
                continue
            return True
    return False


def _ip_literal_candidates(candidate: str) -> list[str]:
    stripped = candidate.strip()
    if not stripped:
        return []

    bracketed = re.fullmatch(r"\[([^\]]+)](?::[0-9]{1,5})?[.!?]*", stripped)
    if bracketed:
        return [bracketed.group(1)]

    normalized = stripped.strip("[](){}<>\"'")
    normalized = normalized.rstrip(".!?")
    if not normalized:
        return []

    return [normalized]


def _bounded_list(raw_value: Any, field: str) -> list[Any]:
    if not isinstance(raw_value, list):
        raise ValueError(f"{field} must be a list")
    if len(raw_value) > MAX_LIST_LENGTH:
        raise ValueError(f"{field} has too many entries")
    return raw_value


def _finite_number(raw_value: Any, field: str) -> float:
    if isinstance(raw_value, bool) or not isinstance(raw_value, int | float):
        raise ValueError(f"{field} must be a finite number")
    value = float(raw_value)
    if not math.isfinite(value):
        raise ValueError(f"{field} must be a finite number")
    return value


def _logistic(value: float) -> float:
    if value >= 0:
        exp_neg = math.exp(-value)
        return 1.0 / (1.0 + exp_neg)
    exp_value = math.exp(value)
    return exp_value / (1.0 + exp_value)


def _round(value: float) -> float:
    rounded = round(value, 6)
    return 0.0 if rounded == 0 else rounded


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Score feature_vector_row.v0 rows with a native inference manifest."
    )
    parser.add_argument("manifest", help="native_inference_manifest.v0 JSON")
    parser.add_argument("feature_rows", help="feature_vector_row.v0 JSON or JSONL")
    parser.add_argument("output", help="Path to write model_score_row.v0 JSON list")
    args = parser.parse_args(argv)

    rows = score_feature_rows(load_manifest(args.manifest), load_feature_rows(args.feature_rows))
    dump_score_rows(rows, args.output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
