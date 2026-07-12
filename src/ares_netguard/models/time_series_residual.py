"""Calibrated offline time-series residual anomaly evidence producer.

The v1 producer uses a numeric-only forecast backend seam and a frozen held-out
calibration cohort. The legacy v0 report remains accepted only as a strict
read-only input contract.
"""

from __future__ import annotations

import argparse
import json
import math
import os
import re
import tempfile
from collections import Counter, defaultdict
from collections.abc import Mapping, Sequence
from datetime import datetime, timedelta
from pathlib import Path
from typing import Any

from ares_netguard.models.disagreement import ROW_SCHEMA_VERSION
from ares_netguard.models.time_series_forecast import (
    CHRONOS_BACKEND_ID,
    CHRONOS_BACKEND_KIND,
    CHRONOS_BACKEND_SETTINGS,
    CHRONOS_BACKEND_VERSION,
    CHRONOS_BUNDLE_SHA256,
    CHRONOS_CONFIG_SHA256,
    CHRONOS_MODEL_ID,
    CHRONOS_MODEL_REVISION,
    CHRONOS_PACKAGE_VERSIONS,
    CHRONOS_RUNTIME_PLATFORM,
    CHRONOS_WEIGHTS_SHA256,
    DEFAULT_BACKEND_NAME,
    SUPPORTED_BACKEND_NAMES,
    ForecastArtifactProvenance,
    ForecastBackend,
    ForecastBackendSafety,
    ForecastEstimate,
    ForecastRequest,
    ForecastSetting,
    PretrainedForecastBackendSafety,
    resolve_forecast_backend,
    validate_forecast_estimate,
    validate_forecast_run_requirements,
)

REPORT_SCHEMA_VERSION = "time_series_residual_report.v1"
PRETRAINED_REPORT_SCHEMA_VERSION = "time_series_residual_report.v2"
LEGACY_REPORT_SCHEMA_VERSION = "time_series_residual_report.v0"
SUPPORTED_REPORT_SCHEMA_VERSIONS = frozenset(
    {
        LEGACY_REPORT_SCHEMA_VERSION,
        REPORT_SCHEMA_VERSION,
        PRETRAINED_REPORT_SCHEMA_VERSION,
    }
)
SUPPORTED_REPORT_SCHEMAS = SUPPORTED_REPORT_SCHEMA_VERSIONS

MODEL_ID = "time_series_residual"
MODEL_FAMILY = "experimental_time_series"
DEFAULT_HISTORY_WINDOW = 3
DEFAULT_CALIBRATION_WINDOW = 8
DEFAULT_INTERVAL_Z = 2.0
MIN_SCALE = 1e-6
MAX_INPUT_ROWS = 20_000
MAX_REPORT_ROWS = 20_000
MAX_SETTING_COUNT = 32
MIN_SERIES_OBSERVATIONS = 12

CALIBRATION_METHOD = "split_conformal_standardized_absolute_residual"
CALIBRATION_TIE_RULE = ">="
PROVENANCE_EVIDENCE_KIND = "forecast_backend_calibration_provenance"

TIME_WINDOW_ROW_FIELDS = frozenset({"entity_id", "feature_name", "window_start", "actual_value"})
LEGACY_REPORT_FIELDS = frozenset(
    {
        "schema_version",
        "model_id",
        "model_family",
        "history_window",
        "interval_z",
        "rows",
    }
)
REPORT_FIELDS = frozenset(
    {
        "schema_version",
        "model_id",
        "model_family",
        "history_window",
        "calibration_window",
        "interval_z",
        "forecast_backend",
        "calibration",
        "safety_flags",
        "rows",
    }
)
FORECAST_BACKEND_FIELDS = frozenset({"backend_id", "backend_version", "backend_kind", "settings"})
PRETRAINED_FORECAST_BACKEND_FIELDS = FORECAST_BACKEND_FIELDS | {"artifact"}
FORECAST_ARTIFACT_FIELDS = frozenset(
    {
        "model_id",
        "revision",
        "license_id",
        "serialization",
        "config_sha256",
        "weights_sha256",
        "bundle_sha256",
        "runtime_platform",
        "packages",
    }
)
CALIBRATION_FIELDS = frozenset(
    {
        "method",
        "count",
        "frozen",
        "tie_rule",
        "finite_sample_correction",
        "score_before_observe",
        "no_future_data",
    }
)
SAFETY_FLAG_FIELDS = frozenset(
    {
        "local_only",
        "synthetic_only",
        "no_pretrained_model",
        "no_artifact",
        "no_network",
        "no_download",
        "no_external_service",
        "no_deployment",
    }
)
PRETRAINED_SAFETY_FLAG_FIELDS = frozenset(
    {
        "local_only",
        "synthetic_only",
        "pretrained_model_used",
        "operator_provisioned_artifact",
        "artifact_digest_verified",
        "local_files_only",
        "network_used",
        "download_used",
        "external_service_used",
        "remote_code_used",
        "artifact_persisted_by_ares",
        "deployment_allowed",
    }
)
CHRONOS_PACKAGE_NAMES = frozenset(CHRONOS_PACKAGE_VERSIONS)
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

SAFE_ENTITY_ID_RE = re.compile(r"^(?:asset|entity|fixture|host|sensor)-[a-z0-9][a-z0-9_-]{0,62}$")
SAFE_FEATURE_NAME_RE = re.compile(r"^[a-z][a-z0-9_]{0,63}$")
SAFE_BACKEND_ID_RE = re.compile(r"^[a-z][a-z0-9_]{0,80}$")
SAFE_BACKEND_VERSION_RE = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._-]{0,31}$")
SAFE_SETTING_KEY_RE = re.compile(r"^[a-z][a-z0-9_]{0,63}$")
SAFE_SETTING_TEXT_RE = re.compile(r"^[a-z0-9][a-z0-9_.-]{0,127}$")
SAFE_PACKAGE_VERSION_RE = re.compile(r"^[A-Za-z0-9][A-Za-z0-9.+_-]{0,63}$")

JsonMap = dict[str, Any]


def load_time_window_rows(path: str | Path) -> list[JsonMap]:
    """Load and strictly validate bounded JSON/JSONL time-window rows."""
    source = Path(path)
    if source.is_dir():
        raise ValueError(f"time-window input must be a file, not a directory: {source}")
    if not source.exists():
        raise ValueError(f"time-window input does not exist: {source}")

    text = source.read_text(encoding="utf-8").strip()
    if not text:
        return []

    if source.suffix.lower() == ".jsonl":
        lines = [line for line in text.splitlines() if line.strip()]
        _validate_row_count(len(lines), "time-window input")
        rows = [_loads_strict(line) for line in lines]
    else:
        payload = _loads_strict(text)
        rows = _rows_from_payload(payload, source)
        _validate_row_count(len(rows), "time-window input")

    normalized = [_normalize_input_row(row) for row in rows]
    _validate_input_order(normalized)
    return [
        {
            "entity_id": row["entity_id"],
            "feature_name": row["feature_name"],
            "window_start": row["window_start"],
            "actual_value": row["actual_value"],
        }
        for row in normalized
    ]


def generate_residual_report(
    rows: Sequence[Mapping[str, Any]],
    *,
    history_window: int = DEFAULT_HISTORY_WINDOW,
    calibration_window: int = DEFAULT_CALIBRATION_WINDOW,
    interval_z: float = DEFAULT_INTERVAL_Z,
    backend: ForecastBackend | None = None,
) -> JsonMap:
    """Generate strict v1/v2 evidence using frozen split-conformal calibration."""
    _validate_settings(history_window, calibration_window, interval_z)
    if isinstance(rows, str | bytes | bytearray) or not isinstance(rows, Sequence):
        raise ValueError("time-window rows must be a sequence of objects")
    _validate_row_count(len(rows), "time-window input")
    if not rows:
        raise ValueError("at least one time-window row is required")

    normalized = [_normalize_input_row(row) for row in rows]
    _validate_input_order(normalized)
    _validate_series_lengths(
        normalized,
        minimum=max(
            MIN_SERIES_OBSERVATIONS,
            history_window + calibration_window + 1,
        ),
    )

    selected_backend = (
        backend if backend is not None else resolve_forecast_backend(DEFAULT_BACKEND_NAME)
    )
    validate_forecast_run_requirements(
        selected_backend,
        history_window=history_window,
        calibration_window=calibration_window,
        interval_z=interval_z,
    )
    report_schema = _report_schema_for_backend(selected_backend)
    backend_metadata = _forecast_backend_metadata(selected_backend, schema=report_schema)
    backend_safety_flags = _forecast_backend_safety_flags(
        selected_backend,
        schema=report_schema,
    )

    history_by_series: dict[tuple[str, str], list[float]] = defaultdict(list)
    calibration_by_series: dict[tuple[str, str], list[float]] = defaultdict(list)
    residual_rows: list[JsonMap] = []

    for row in normalized:
        series_key = (row["entity_id"], row["feature_name"])
        history = history_by_series[series_key]
        if len(history) < history_window:
            history.append(row["actual_value"])
            continue

        estimate = _forecast_one(
            selected_backend,
            history[-history_window:],
            interval_z=interval_z,
        )
        actual = row["actual_value"]
        residual = _finite_number(
            actual - estimate.mean,
            "forecast calibration residual",
        )
        standardized_abs_residual = _finite_number(
            abs(residual) / estimate.scale,
            "forecast calibration standardized absolute residual",
        )
        calibration_scores = calibration_by_series[series_key]

        if len(calibration_scores) < calibration_window:
            calibration_scores.append(standardized_abs_residual)
        else:
            evidence = _build_residual_row(
                row,
                estimate,
                calibration_scores=tuple(calibration_scores),
            )
            validate_residual_evidence_row(evidence)
            residual_rows.append(evidence)

        # The current target becomes history only after its forecast, calibration,
        # or anomaly score has been computed.
        history.append(actual)

    residual_rows.sort(
        key=lambda item: (item["entity_id"], item["feature_name"], item["window_start"])
    )
    report = {
        "schema_version": report_schema,
        "model_id": MODEL_ID,
        "model_family": MODEL_FAMILY,
        "history_window": history_window,
        "calibration_window": calibration_window,
        "interval_z": _round(interval_z),
        "forecast_backend": backend_metadata,
        "calibration": _calibration_metadata(calibration_window),
        "safety_flags": backend_safety_flags,
        "rows": residual_rows,
    }
    validate_residual_report(report)
    return report


def residual_evidence_to_score_rows(
    report_or_rows: Mapping[str, Any] | Sequence[Mapping[str, Any]],
) -> list[JsonMap]:
    """Convert strict v0/v1 residual evidence into model_score_row.v0 rows."""
    residual_rows, provenance = _extract_residual_rows(report_or_rows)
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
        evidence: list[JsonMap] = [dict(row) for row in evidence_rows]
        if provenance is not None:
            evidence.append(_clone_json(provenance))
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
                        "evidence": evidence,
                    }
                },
            }
        )
    return score_rows


def validate_residual_report(report: Mapping[str, Any]) -> None:
    """Validate the complete strict v0, v1, or v2 residual report contract."""
    if not isinstance(report, Mapping):
        raise ValueError("residual report must be an object")
    schema = report.get("schema_version")
    if schema == LEGACY_REPORT_SCHEMA_VERSION:
        _require_exact_fields(report, LEGACY_REPORT_FIELDS, "legacy residual report")
    elif schema in {REPORT_SCHEMA_VERSION, PRETRAINED_REPORT_SCHEMA_VERSION}:
        _require_exact_fields(report, REPORT_FIELDS, "residual report")
    else:
        supported = ", ".join(sorted(SUPPORTED_REPORT_SCHEMA_VERSIONS))
        raise ValueError(f"residual report schema_version must be one of: {supported}")

    if report["model_id"] != MODEL_ID:
        raise ValueError(f"residual report model_id must be '{MODEL_ID}'")
    if report["model_family"] != MODEL_FAMILY:
        raise ValueError(f"residual report model_family must be '{MODEL_FAMILY}'")
    history_window = _positive_int(report["history_window"], "history_window")
    _bounded_number(report["interval_z"], "interval_z", MIN_SCALE, 1000.0)

    if schema in {REPORT_SCHEMA_VERSION, PRETRAINED_REPORT_SCHEMA_VERSION}:
        calibration_window = _positive_int(report["calibration_window"], "calibration_window")
        _validate_backend_metadata(
            report["forecast_backend"],
            pretrained=schema == PRETRAINED_REPORT_SCHEMA_VERSION,
        )
        _validate_calibration_metadata(report["calibration"], calibration_window=calibration_window)
        if schema == PRETRAINED_REPORT_SCHEMA_VERSION:
            _validate_pretrained_safety_flags(report["safety_flags"])
            if history_window != 64 or calibration_window != 32 or report["interval_z"] != 2.0:
                raise ValueError("v2 residual report requires the fixed 64/32/2.0 run settings")
        else:
            _validate_safety_flags(report["safety_flags"])
        if history_window + calibration_window + 1 > MAX_INPUT_ROWS:
            raise ValueError("history and calibration windows exceed the input row bound")

    rows = report["rows"]
    if not isinstance(rows, list):
        raise ValueError("residual report requires a 'rows' list")
    if len(rows) > MAX_REPORT_ROWS:
        raise ValueError(f"residual report rows must not exceed {MAX_REPORT_ROWS}")
    if schema in {REPORT_SCHEMA_VERSION, PRETRAINED_REPORT_SCHEMA_VERSION} and not rows:
        raise ValueError("v1/v2 residual report rows must not be empty")

    row_keys: list[tuple[str, str, str]] = []
    for row in rows:
        validate_residual_evidence_row(row)
        row_keys.append((row["entity_id"], row["feature_name"], row["window_start"]))
    if row_keys != sorted(row_keys):
        raise ValueError("residual report rows must be sorted by entity, feature, and window")
    if len(row_keys) != len(set(row_keys)):
        raise ValueError("residual report rows must not contain duplicate series windows")


def validate_residual_evidence_row(row: Mapping[str, Any]) -> None:
    """Validate the unchanged residual evidence row schema."""
    if not isinstance(row, Mapping):
        raise ValueError("residual evidence row must be an object")
    _require_exact_fields(row, frozenset(RESIDUAL_ROW_FIELDS), "residual evidence row")

    _validate_entity_id(row["entity_id"])
    _validate_feature_name(row["feature_name"])
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

    if not row["forecast_lower"] <= row["forecast_mean"] <= row["forecast_upper"]:
        raise ValueError(
            "residual evidence requires forecast_lower <= forecast_mean <= forecast_upper"
        )
    _bounded_number(row["conformal_score"], "conformal_score", 0.0, 1.0)
    _bounded_number(row["residual_risk"], "residual_risk", 0.0, 1.0)

    if row["model_id"] != MODEL_ID:
        raise ValueError(f"model_id must be '{MODEL_ID}'")
    if row["model_family"] != MODEL_FAMILY:
        raise ValueError(f"model_family must be '{MODEL_FAMILY}'")


def dump_report(
    report: Mapping[str, Any],
    path: str | Path,
    *,
    repo_root: str | Path | None = None,
) -> None:
    """Validate and atomically write one strict v0/v1/v2 residual report."""
    validate_residual_report(report)
    output = _validated_output_path(path, repo_root=repo_root)
    serialized = json.dumps(report, allow_nan=False, indent=2, sort_keys=True) + "\n"
    _atomic_write_text(output, serialized)


def _extract_residual_rows(
    report_or_rows: Mapping[str, Any] | Sequence[Mapping[str, Any]],
) -> tuple[Sequence[Mapping[str, Any]], JsonMap | None]:
    if isinstance(report_or_rows, Mapping):
        validate_residual_report(report_or_rows)
        provenance = None
        if report_or_rows["schema_version"] in {
            REPORT_SCHEMA_VERSION,
            PRETRAINED_REPORT_SCHEMA_VERSION,
        }:
            provenance = {
                "evidence_kind": PROVENANCE_EVIDENCE_KIND,
                "forecast_backend": _clone_json(report_or_rows["forecast_backend"]),
                "calibration": _clone_json(report_or_rows["calibration"]),
            }
            if report_or_rows["schema_version"] == PRETRAINED_REPORT_SCHEMA_VERSION:
                provenance["safety_flags"] = _clone_json(report_or_rows["safety_flags"])
        return report_or_rows["rows"], provenance

    if isinstance(report_or_rows, str | bytes | bytearray) or not isinstance(
        report_or_rows, Sequence
    ):
        raise ValueError("residual evidence input must be a report object or row sequence")
    if len(report_or_rows) > MAX_REPORT_ROWS:
        raise ValueError(f"residual evidence rows must not exceed {MAX_REPORT_ROWS}")
    return report_or_rows, None


def _forecast_one(
    backend: ForecastBackend,
    context: Sequence[float],
    *,
    interval_z: float,
) -> ForecastEstimate:
    forecast_one = getattr(backend, "forecast_one", None)
    if not callable(forecast_one):
        raise ValueError("forecast backend requires callable forecast_one")
    request = ForecastRequest(
        context=tuple(_finite_number(value, "forecast context value") for value in context),
        interval_z=_finite_number(interval_z, "interval_z"),
    )
    return validate_forecast_estimate(forecast_one(request))


def _build_residual_row(
    row: Mapping[str, Any],
    estimate: ForecastEstimate,
    *,
    calibration_scores: Sequence[float],
) -> JsonMap:
    actual = row["actual_value"]
    residual = _finite_number(actual - estimate.mean, "forecast target residual")
    residual_z = _finite_number(
        residual / estimate.scale,
        "forecast target standardized residual",
    )
    target_score = abs(residual_z)
    tail_count = sum(score >= target_score for score in calibration_scores)
    p_value = (1.0 + tail_count) / (len(calibration_scores) + 1.0)
    conformal_score = 1.0 - p_value
    z_risk = min(1.0, target_score / 4.0)
    interval_breached = actual < estimate.lower or actual > estimate.upper
    residual_risk = max(conformal_score, z_risk, 0.75 if interval_breached else 0.0)

    return {
        "entity_id": row["entity_id"],
        "feature_name": row["feature_name"],
        "window_start": row["window_start"],
        "actual_value": _round(actual),
        "forecast_mean": _round(estimate.mean),
        "forecast_lower": _round(estimate.lower),
        "forecast_upper": _round(estimate.upper),
        "residual": _round(residual),
        "residual_z": _round(residual_z),
        "conformal_score": _round(conformal_score),
        "residual_risk": _round(residual_risk),
        "model_id": MODEL_ID,
        "model_family": MODEL_FAMILY,
    }


def _report_schema_for_backend(backend: ForecastBackend) -> str:
    safety = getattr(backend, "safety", None)
    artifact = getattr(backend, "artifact", None)
    if isinstance(safety, ForecastBackendSafety) and artifact is None:
        return REPORT_SCHEMA_VERSION
    if isinstance(safety, PretrainedForecastBackendSafety) and isinstance(
        artifact, ForecastArtifactProvenance
    ):
        return PRETRAINED_REPORT_SCHEMA_VERSION
    raise ValueError("forecast backend safety and artifact contracts are inconsistent")


def _forecast_backend_metadata(backend: ForecastBackend, *, schema: str) -> JsonMap:
    try:
        settings = backend.settings
        metadata = {
            "backend_id": backend.backend_id,
            "backend_version": backend.backend_version,
            "backend_kind": backend.backend_kind,
            "settings": _normalized_backend_settings(settings),
        }
    except AttributeError as exc:
        raise ValueError("forecast backend identity and settings are required") from exc
    if schema == PRETRAINED_REPORT_SCHEMA_VERSION:
        artifact = getattr(backend, "artifact", None)
        if not isinstance(artifact, ForecastArtifactProvenance):
            raise ValueError("pretrained forecast backend requires artifact provenance")
        metadata["artifact"] = _forecast_artifact_metadata(artifact)
    _validate_backend_metadata(
        metadata,
        pretrained=schema == PRETRAINED_REPORT_SCHEMA_VERSION,
    )
    return metadata


def _forecast_artifact_metadata(artifact: ForecastArtifactProvenance) -> JsonMap:
    metadata = {
        "model_id": artifact.model_id,
        "revision": artifact.revision,
        "license_id": artifact.license_id,
        "serialization": artifact.serialization,
        "config_sha256": artifact.config_sha256,
        "weights_sha256": artifact.weights_sha256,
        "bundle_sha256": artifact.bundle_sha256,
        "runtime_platform": artifact.runtime_platform,
        "packages": dict(sorted(artifact.packages.items())),
    }
    _validate_forecast_artifact(metadata)
    return metadata


def _normalized_backend_settings(settings: Mapping[str, ForecastSetting]) -> JsonMap:
    if not isinstance(settings, Mapping):
        raise ValueError("forecast backend settings must be an immutable-style mapping")
    if len(settings) > MAX_SETTING_COUNT:
        raise ValueError(f"forecast backend settings must not exceed {MAX_SETTING_COUNT} entries")
    normalized: JsonMap = {}
    for key, value in sorted(settings.items()):
        if not isinstance(key, str) or not SAFE_SETTING_KEY_RE.fullmatch(key):
            raise ValueError("forecast backend setting keys must be snake_case identifiers")
        if isinstance(value, bool):
            normalized[key] = value
        elif isinstance(value, int | float):
            normalized[key] = _finite_number(value, f"forecast backend setting {key}")
        elif isinstance(value, str) and SAFE_SETTING_TEXT_RE.fullmatch(value):
            normalized[key] = value
        else:
            raise ValueError("forecast backend setting values must be safe JSON scalars")
    return normalized


def _calibration_metadata(calibration_window: int) -> JsonMap:
    return {
        "method": CALIBRATION_METHOD,
        "count": calibration_window,
        "frozen": True,
        "tie_rule": CALIBRATION_TIE_RULE,
        "finite_sample_correction": True,
        "score_before_observe": True,
        "no_future_data": True,
    }


def _forecast_backend_safety_flags(backend: ForecastBackend, *, schema: str) -> JsonMap:
    try:
        safety = backend.safety
    except AttributeError as exc:
        raise ValueError("forecast backend requires an immutable safety contract") from exc
    if schema == REPORT_SCHEMA_VERSION:
        if not isinstance(safety, ForecastBackendSafety):
            raise ValueError("v1 forecast backend safety contract is malformed")
        flags = {
            "local_only": safety.local_only,
            "synthetic_only": safety.synthetic_only,
            "no_pretrained_model": safety.no_pretrained_model,
            "no_artifact": safety.no_artifact,
            "no_network": safety.no_network,
            "no_download": safety.no_download,
            "no_external_service": safety.no_external_service,
            "no_deployment": safety.no_deployment,
        }
        _validate_safety_flags(flags)
        return flags
    if schema != PRETRAINED_REPORT_SCHEMA_VERSION or not isinstance(
        safety, PretrainedForecastBackendSafety
    ):
        raise ValueError("v2 forecast backend safety contract is malformed")
    flags = {
        "local_only": safety.local_only,
        "synthetic_only": safety.synthetic_only,
        "pretrained_model_used": safety.pretrained_model_used,
        "operator_provisioned_artifact": safety.operator_provisioned_artifact,
        "artifact_digest_verified": safety.artifact_digest_verified,
        "local_files_only": safety.local_files_only,
        "network_used": safety.network_used,
        "download_used": safety.download_used,
        "external_service_used": safety.external_service_used,
        "remote_code_used": safety.remote_code_used,
        "artifact_persisted_by_ares": safety.artifact_persisted_by_ares,
        "deployment_allowed": safety.deployment_allowed,
    }
    _validate_pretrained_safety_flags(flags)
    return flags


def _validate_backend_metadata(raw_metadata: Any, *, pretrained: bool) -> None:
    if not isinstance(raw_metadata, Mapping):
        raise ValueError("forecast_backend must be an object")
    expected_fields = PRETRAINED_FORECAST_BACKEND_FIELDS if pretrained else FORECAST_BACKEND_FIELDS
    _require_exact_fields(raw_metadata, expected_fields, "forecast_backend")
    backend_id = raw_metadata["backend_id"]
    version = raw_metadata["backend_version"]
    kind = raw_metadata["backend_kind"]
    if not isinstance(backend_id, str) or not SAFE_BACKEND_ID_RE.fullmatch(backend_id):
        raise ValueError("forecast_backend.backend_id must be a safe identifier")
    if not isinstance(version, str) or not SAFE_BACKEND_VERSION_RE.fullmatch(version):
        raise ValueError("forecast_backend.backend_version must be a safe version")
    if not isinstance(kind, str) or not SAFE_BACKEND_ID_RE.fullmatch(kind):
        raise ValueError("forecast_backend.backend_kind must be a safe identifier")
    normalized = _normalized_backend_settings(raw_metadata["settings"])
    if dict(raw_metadata["settings"]) != normalized:
        raise ValueError("forecast_backend.settings must contain normalized safe scalars")
    if pretrained:
        if backend_id != CHRONOS_BACKEND_ID:
            raise ValueError(f"v2 forecast_backend.backend_id must be '{CHRONOS_BACKEND_ID}'")
        if version != CHRONOS_BACKEND_VERSION or kind != CHRONOS_BACKEND_KIND:
            raise ValueError("v2 forecast_backend version and kind must be pinned")
        if normalized != dict(CHRONOS_BACKEND_SETTINGS):
            raise ValueError("v2 forecast_backend settings must be pinned")
        _validate_forecast_artifact(raw_metadata["artifact"])


def _validate_forecast_artifact(raw_artifact: Any) -> None:
    if not isinstance(raw_artifact, Mapping):
        raise ValueError("forecast_backend.artifact must be an object")
    _require_exact_fields(raw_artifact, FORECAST_ARTIFACT_FIELDS, "forecast_backend.artifact")
    expected_scalars = {
        "model_id": CHRONOS_MODEL_ID,
        "revision": CHRONOS_MODEL_REVISION,
        "license_id": "apache-2.0",
        "serialization": "safetensors",
        "config_sha256": CHRONOS_CONFIG_SHA256,
        "weights_sha256": CHRONOS_WEIGHTS_SHA256,
        "runtime_platform": CHRONOS_RUNTIME_PLATFORM,
    }
    for field, expected in expected_scalars.items():
        if raw_artifact[field] != expected:
            raise ValueError(f"forecast_backend.artifact.{field} is not pinned")
    if raw_artifact["bundle_sha256"] != CHRONOS_BUNDLE_SHA256:
        raise ValueError("forecast_backend.artifact.bundle_sha256 is not pinned")
    packages = raw_artifact["packages"]
    if not isinstance(packages, Mapping):
        raise ValueError("forecast_backend.artifact.packages must be an object")
    _require_exact_fields(packages, CHRONOS_PACKAGE_NAMES, "forecast_backend.artifact.packages")
    for version in packages.values():
        if not isinstance(version, str) or not SAFE_PACKAGE_VERSION_RE.fullmatch(version):
            raise ValueError("forecast_backend.artifact package versions must be safe")
    if dict(packages) != dict(CHRONOS_PACKAGE_VERSIONS):
        raise ValueError("forecast_backend.artifact package versions are not pinned")


def _validate_calibration_metadata(raw_metadata: Any, *, calibration_window: int) -> None:
    if not isinstance(raw_metadata, Mapping):
        raise ValueError("calibration must be an object")
    _require_exact_fields(raw_metadata, CALIBRATION_FIELDS, "calibration")
    if raw_metadata["method"] != CALIBRATION_METHOD:
        raise ValueError(f"calibration.method must be '{CALIBRATION_METHOD}'")
    if _positive_int(raw_metadata["count"], "calibration.count") != calibration_window:
        raise ValueError("calibration.count must match calibration_window")
    if raw_metadata["tie_rule"] != CALIBRATION_TIE_RULE:
        raise ValueError(f"calibration.tie_rule must be '{CALIBRATION_TIE_RULE}'")
    for field in (
        "frozen",
        "finite_sample_correction",
        "score_before_observe",
        "no_future_data",
    ):
        if raw_metadata[field] is not True:
            raise ValueError(f"calibration.{field} must be true")


def _validate_safety_flags(raw_flags: Any) -> None:
    if not isinstance(raw_flags, Mapping):
        raise ValueError("safety_flags must be an object")
    _require_exact_fields(raw_flags, SAFETY_FLAG_FIELDS, "safety_flags")
    for field in sorted(SAFETY_FLAG_FIELDS):
        if raw_flags[field] is not True:
            raise ValueError(f"safety_flags.{field} must be true")


def _validate_pretrained_safety_flags(raw_flags: Any) -> None:
    if not isinstance(raw_flags, Mapping):
        raise ValueError("safety_flags must be an object")
    _require_exact_fields(raw_flags, PRETRAINED_SAFETY_FLAG_FIELDS, "safety_flags")
    required_true = {
        "local_only",
        "synthetic_only",
        "pretrained_model_used",
        "operator_provisioned_artifact",
        "artifact_digest_verified",
        "local_files_only",
    }
    required_false = PRETRAINED_SAFETY_FLAG_FIELDS - required_true
    for field in sorted(required_true):
        if raw_flags[field] is not True:
            raise ValueError(f"safety_flags.{field} must be true")
    for field in sorted(required_false):
        if raw_flags[field] is not False:
            raise ValueError(f"safety_flags.{field} must be false")


def _rows_from_payload(payload: Any, source: Path) -> list[Any]:
    if isinstance(payload, list):
        return payload
    if isinstance(payload, Mapping) and "rows" in payload:
        _require_exact_fields(payload, frozenset({"rows"}), "time-window payload")
        if not isinstance(payload["rows"], list):
            raise ValueError("time-window payload rows must be a list")
        return payload["rows"]
    if isinstance(payload, Mapping):
        return [payload]
    raise ValueError(f"unsupported time-window row payload in {source}")


def _normalize_input_row(row: Mapping[str, Any]) -> JsonMap:
    if not isinstance(row, Mapping):
        raise ValueError("time-window row must be an object")
    _require_exact_fields(row, TIME_WINDOW_ROW_FIELDS, "time-window row")

    entity_id = _validate_entity_id(row["entity_id"])
    feature_name = _validate_feature_name(row["feature_name"])
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


def _validate_series_lengths(rows: Sequence[Mapping[str, Any]], *, minimum: int) -> None:
    counts = Counter((row["entity_id"], row["feature_name"]) for row in rows)
    for (entity_id, feature_name), count in sorted(counts.items()):
        if count < minimum:
            raise ValueError(
                f"series {entity_id}/{feature_name} requires at least {minimum} observations; "
                f"received {count}"
            )


def _validate_settings(
    history_window: int,
    calibration_window: int,
    interval_z: float,
) -> None:
    history = _positive_int(history_window, "history_window")
    calibration = _positive_int(calibration_window, "calibration_window")
    _bounded_number(interval_z, "interval_z", MIN_SCALE, 1000.0)
    if history + calibration + 1 > MAX_INPUT_ROWS:
        raise ValueError("history and calibration windows exceed the input row bound")


def _validate_entity_id(raw_value: Any) -> str:
    if not isinstance(raw_value, str) or not SAFE_ENTITY_ID_RE.fullmatch(raw_value):
        raise ValueError("entity_id must be a safe coarse identifier")
    return raw_value


def _validate_feature_name(raw_value: Any) -> str:
    if not isinstance(raw_value, str) or not SAFE_FEATURE_NAME_RE.fullmatch(raw_value):
        raise ValueError("feature_name must be a snake_case identifier")
    return raw_value


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
    if parsed.tzinfo is None or parsed.utcoffset() != timedelta(0):
        raise ValueError("window_start must be a UTC timestamp")
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


def _positive_int(raw_value: Any, field: str) -> int:
    if isinstance(raw_value, bool) or not isinstance(raw_value, int) or raw_value < 1:
        raise ValueError(f"{field} must be a positive integer")
    return raw_value


def _validate_row_count(count: int, label: str) -> None:
    if count > MAX_INPUT_ROWS:
        raise ValueError(f"{label} must not exceed {MAX_INPUT_ROWS} rows")


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

    def reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> JsonMap:
        result: JsonMap = {}
        for key, value in pairs:
            if key in result:
                raise ValueError(f"duplicate JSON object key '{key}' is not allowed")
            result[key] = value
        return result

    return json.loads(
        text,
        parse_constant=reject_constant,
        object_pairs_hook=reject_duplicate_keys,
    )


def _clone_json(value: Any) -> Any:
    return json.loads(json.dumps(value, allow_nan=False, sort_keys=True))


def _round(value: float) -> float:
    rounded = round(value, 6)
    return 0.0 if rounded == 0 else rounded


def _validated_output_path(path: str | Path, *, repo_root: str | Path | None = None) -> Path:
    output = Path(path)
    if output.is_dir():
        raise ValueError(f"output path must be a file, not a directory: {output}")
    if output.is_symlink():
        raise ValueError("output path must not be a symlink")

    lexical = Path(os.path.abspath(output))
    resolved = output.resolve(strict=False)
    repo = Path(repo_root).resolve() if repo_root is not None else _repository_root()
    lexical_allowed_roots = (
        repo / "data" / "reports",
        repo / ".runtime",
        repo / "artifacts",
    )
    resolved_allowed_roots = tuple(
        candidate
        for root in lexical_allowed_roots
        if _is_relative_to((candidate := root.resolve(strict=False)), repo)
    )
    lexical_inside_repo = _is_relative_to(lexical, repo)
    lexical_allowed = any(_is_relative_to(lexical, root) for root in lexical_allowed_roots)
    resolved_inside_repo = _is_relative_to(resolved, repo)
    resolved_allowed = any(_is_relative_to(resolved, root) for root in resolved_allowed_roots)
    if (lexical_inside_repo and not (lexical_allowed and resolved_allowed)) or (
        resolved_inside_repo and not resolved_allowed
    ):
        raise ValueError(
            "repository output paths must be under data/reports/, .runtime/, or artifacts/"
        )
    return output


def _repository_root() -> Path:
    for parent in Path(__file__).resolve().parents:
        if (parent / "AGENTS.md").is_file() and (parent / "pyproject.toml").is_file():
            return parent.resolve()
    return Path.cwd().resolve()


def _is_relative_to(child: Path, parent: Path) -> bool:
    try:
        child.relative_to(parent)
    except ValueError:
        return False
    return True


def _atomic_write_text(output: Path, serialized: str) -> None:
    temporary_path: Path | None = None
    try:
        with tempfile.NamedTemporaryFile(
            mode="w",
            encoding="utf-8",
            dir=output.parent,
            prefix=f".{output.name}.",
            suffix=".tmp",
            delete=False,
        ) as handle:
            handle.write(serialized)
            handle.flush()
            os.fsync(handle.fileno())
            temporary_path = Path(handle.name)
        os.replace(temporary_path, output)
        temporary_path = None
    finally:
        if temporary_path is not None:
            temporary_path.unlink(missing_ok=True)


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Generate calibrated offline time-series residual evidence."
    )
    parser.add_argument("input", help="Strict JSON or JSONL time-window feature rows")
    parser.add_argument("output", help="Path to atomically write residual report JSON")
    parser.add_argument(
        "--backend",
        default=DEFAULT_BACKEND_NAME,
        help=f"Closed forecast backend selector (allowed: {', '.join(SUPPORTED_BACKEND_NAMES)})",
    )
    parser.add_argument(
        "--model-root",
        help="Explicit local model root; required only for chronos_bolt_tiny_local",
    )
    parser.add_argument(
        "--history-window",
        type=int,
        default=DEFAULT_HISTORY_WINDOW,
        help="Number of past values supplied to each forecast request",
    )
    parser.add_argument(
        "--calibration-window",
        type=int,
        default=DEFAULT_CALIBRATION_WINDOW,
        help="Number of held-out residual scores frozen for calibration",
    )
    parser.add_argument(
        "--interval-z",
        type=float,
        default=DEFAULT_INTERVAL_Z,
        help="Forecast interval width in backend standardization scales",
    )
    args = parser.parse_args(argv)

    backend = resolve_forecast_backend(args.backend, model_root=args.model_root)
    report = generate_residual_report(
        load_time_window_rows(args.input),
        history_window=args.history_window,
        calibration_window=args.calibration_window,
        interval_z=args.interval_z,
        backend=backend,
    )
    dump_report(report, args.output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
