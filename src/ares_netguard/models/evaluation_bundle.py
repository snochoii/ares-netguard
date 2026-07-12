"""Local model evaluation bundle contract.

The v0 bundle summarizes already-generated synthetic model reports and score
rows. It does not train models, load model artifacts, inspect telemetry,
perform capture, call external services, deploy rules, or promote models.
"""

from __future__ import annotations

import argparse
import ipaddress
import json
import math
import re
from collections import Counter
from collections.abc import Mapping, Sequence
from datetime import datetime
from pathlib import Path
from typing import Any

from ares_netguard.detection_engineering import candidates as detection_candidates
from ares_netguard.graph import temporal_security_graph
from ares_netguard.investigation import agentic_layer
from ares_netguard.models import (
    self_supervised_representation,
    time_series_forecast_evaluation,
    time_series_residual,
)
from ares_netguard.models.disagreement import REPORT_SCHEMA_VERSION as DISAGREEMENT_SCHEMA_VERSION
from ares_netguard.models.disagreement import ROW_SCHEMA_VERSION

REPORT_SCHEMA_VERSION = "model_evaluation_bundle.v0"
SCORE_ROWS_SCHEMA_VERSION = "model_score_rows.v0"

SUPPORTED_REPORT_SCHEMAS = frozenset(
    {
        DISAGREEMENT_SCHEMA_VERSION,
        *time_series_residual.SUPPORTED_REPORT_SCHEMA_VERSIONS,
        time_series_forecast_evaluation.REPORT_SCHEMA_VERSION,
        self_supervised_representation.REPORT_SCHEMA_VERSION,
        temporal_security_graph.REPORT_SCHEMA_VERSION,
        agentic_layer.REPORT_SCHEMA_VERSION,
        detection_candidates.REPORT_SCHEMA_VERSION,
    }
)
SUPPORTED_SOURCE_SCHEMAS = SUPPORTED_REPORT_SCHEMAS | {SCORE_ROWS_SCHEMA_VERSION}

MAX_STRING_LENGTH = 2048
MAX_LIST_LENGTH = 20000
MAX_MAPPING_LENGTH = 256
MAX_DEPTH = 18

BUNDLE_FIELDS = frozenset(
    {
        "schema_version",
        "bundle_scope",
        "source_summaries",
        "aggregate_summary",
        "safety_flags",
        "non_claims",
    }
)
SOURCE_SUMMARY_FIELDS = frozenset(
    {
        "source_name",
        "source_schema",
        "row_count",
        "score_row_count",
        "entity_count",
        "window_count",
        "model_ids",
        "feature_count",
        "sequence_count",
        "edge_observation_count",
        "hypothesis_count",
        "candidate_count",
        "candidate_languages",
        "candidate_kinds",
    }
)
AGGREGATE_SUMMARY_FIELDS = frozenset(
    {
        "source_count",
        "schemas_present",
        "source_count_by_schema",
        "row_count_by_schema",
        "score_row_count",
        "entity_count",
        "window_count",
        "model_ids",
        "feature_count",
        "sequence_count",
        "edge_observation_count",
        "hypothesis_count",
        "candidate_count",
        "candidate_languages",
        "candidate_kinds",
    }
)
SAFETY_FLAG_FIELDS = frozenset(
    {
        "local_only",
        "strict_json_loaded",
        "input_paths_copied",
        "source_filenames_copied",
        "raw_identifiers_copied",
        "generated_artifact_references_copied",
        "secrets_detected",
        "report_payload_copied",
        "live_capture_used",
        "external_services_used",
        "deployment_allowed",
    }
)

JsonMap = dict[str, Any]
SourcePayload = Mapping[str, Any] | Sequence[Mapping[str, Any]]

SAFE_SOURCE_NAME_RE = re.compile(r"^[a-z][a-z0-9_]{0,96}_[0-9]{3}$")
SAFE_ENTITY_ID_RE = re.compile(r"^(?:asset|entity|fixture|host|sensor)-[a-z0-9][a-z0-9_-]{0,62}$")
SAFE_MODEL_ID_RE = re.compile(r"^[a-z][a-z0-9_-]{0,80}$")
SAFE_FEATURE_NAME_RE = re.compile(r"^[a-z][a-z0-9_]{0,63}$")
SAFE_FIELD_PATH_RE = re.compile(r"^\[[A-Za-z0-9_-]+](?:\[[A-Za-z0-9_-]+])*$")

URL_RE = re.compile(r"(?i)\b(?:https?|ftp)://")
EMAIL_RE = re.compile(r"(?i)\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b")
IPV4_RE = re.compile(r"\b(?:\d{1,3}\.){3}\d{1,3}\b")
DOMAIN_RE = re.compile(
    r"(?i)\b[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?"
    r"(?:\.[a-z](?:[a-z0-9-]{0,61}[a-z0-9])?)+\b"
)
PATH_RE = re.compile(r"(?i)(?:^|[\s=])(?:/[a-z0-9._-]+){2,}|\b[a-z]:\\")
SECRET_RE = re.compile(r"(?i)\b(?:password|passwd|credential|secret|api[_-]?key|private[_-]?key)\b")
COMMAND_LINE_RE = re.compile(
    r"(?i)(?:^|\s)(?:bash|sh|cmd(?:\.exe)?|powershell|pwsh|curl|wget)\s"
    r"|[;&|]{2}|`"
)
ARTIFACT_EXT_RE = re.compile(
    r"(?i)\.(?:pcapng?|parquet|joblib|pkl|onnx|pt|pth|ckpt|db|sqlite|duckdb|jsonl)\b"
)
SECRET_KEY_RE = re.compile(
    r"(?i)(?:^|_)(?:password|passwd|credential|secret|api_key|apikey|private_key|"
    r"access_token|auth_token|bearer_token)(?:_|$)"
)

DISAGREEMENT_REPORT_FIELDS = frozenset(
    {
        "schema_version",
        "model_score_matrix",
        "model_agreement_score",
        "model_disagreement_score",
        "consensus_risk",
        "outlier_model",
        "outlier_models",
        "top_supporting_models",
        "top_dissenting_models",
        "evidence_by_model",
        "row_reports",
    }
)
DISAGREEMENT_ROW_FIELDS = frozenset(
    {
        "schema_version",
        "entity_id",
        "window_start",
        "scores",
        "agreement_score",
        "disagreement_score",
        "consensus_risk",
        "outlier_model",
        "outlier_models",
        "evidence_by_model",
    }
)
REPRESENTATION_REPORT_FIELDS = frozenset(
    {
        "schema_version",
        "model_id",
        "model_family",
        "embedding_dimensions",
        "sequence_count",
        "token_field_order",
        "rows",
    }
)
GRAPH_REPORT_FIELDS = frozenset(
    {
        "schema_version",
        "model_id",
        "model_family",
        "history_window",
        "min_history_windows",
        "edge_observation_count",
        "rows",
    }
)
SCORE_ROW_FIELDS = frozenset({"schema_version", "entity_id", "window_start", "scores"})


def load_bundle_source(path: str | Path) -> SourcePayload:
    """Load one local JSON report or JSON score-row list using strict constants."""
    source = _input_file(path)
    payload = _loads_strict(source.read_text(encoding="utf-8"))
    if isinstance(payload, Mapping):
        return dict(payload)
    if isinstance(payload, list):
        rows: list[JsonMap] = []
        for index, row in enumerate(payload):
            if not isinstance(row, Mapping):
                raise ValueError(f"score row list entry {index} must be an object")
            rows.append(dict(row))
        return rows
    raise ValueError(f"bundle source must be a JSON object or list: {source}")


def load_bundle_sources(paths: Sequence[str | Path]) -> list[SourcePayload]:
    """Load local JSON reports and JSON score-row lists for bundle generation."""
    return [load_bundle_source(path) for path in paths]


def generate_evaluation_bundle(sources: Sequence[SourcePayload]) -> JsonMap:
    """Generate a deterministic aggregate-only evaluation bundle."""
    if not sources:
        raise ValueError("at least one bundle source is required")

    summaries: list[JsonMap] = []
    schema_counts: Counter[str] = Counter()

    for source in sources:
        schema = _source_schema(source)
        schema_counts[schema] += 1
        source_name = _source_name(schema, schema_counts[schema])
        summaries.append(_summarize_source(source, schema=schema, source_name=source_name))

    bundle = {
        "schema_version": REPORT_SCHEMA_VERSION,
        "bundle_scope": "local_synthetic_model_evaluation_reports",
        "source_summaries": summaries,
        "aggregate_summary": _aggregate_summaries(summaries),
        "safety_flags": {
            "local_only": True,
            "strict_json_loaded": True,
            "input_paths_copied": False,
            "source_filenames_copied": False,
            "raw_identifiers_copied": False,
            "generated_artifact_references_copied": False,
            "secrets_detected": False,
            "report_payload_copied": False,
            "live_capture_used": False,
            "external_services_used": False,
            "deployment_allowed": False,
        },
        "non_claims": [
            "not_model_registry",
            "not_model_promotion_gate",
            "not_live_capture",
            "not_external_enrichment",
            "not_rule_deployment",
            "not_native_runtime_execution",
        ],
    }
    validate_evaluation_bundle(bundle)
    return bundle


def dump_bundle(
    bundle: Mapping[str, Any],
    path: str | Path,
    *,
    repo_root: str | Path | None = None,
) -> None:
    """Write a validated bundle to a non-committed output location."""
    output = _validated_output_path(path, repo_root=repo_root)
    validate_evaluation_bundle(bundle)
    output.write_text(
        json.dumps(bundle, allow_nan=False, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def validate_evaluation_bundle(bundle: Mapping[str, Any]) -> None:
    """Validate the strict model_evaluation_bundle.v0 output contract."""
    if not isinstance(bundle, Mapping):
        raise ValueError("evaluation bundle must be an object")
    _require_exact_fields(bundle, BUNDLE_FIELDS, "evaluation bundle")
    _validate_safe_tree(bundle, "evaluation bundle")

    if bundle["schema_version"] != REPORT_SCHEMA_VERSION:
        raise ValueError(f"evaluation bundle requires schema_version '{REPORT_SCHEMA_VERSION}'")
    if bundle["bundle_scope"] != "local_synthetic_model_evaluation_reports":
        raise ValueError("bundle_scope must be local_synthetic_model_evaluation_reports")

    source_summaries = _bounded_list(bundle["source_summaries"], "source_summaries")
    if not source_summaries:
        raise ValueError("source_summaries must not be empty")
    for summary in source_summaries:
        _validate_source_summary(summary)

    _validate_aggregate_summary(bundle["aggregate_summary"])
    if bundle["aggregate_summary"] != _aggregate_summaries(source_summaries):
        raise ValueError("aggregate_summary must be derived from source_summaries")
    _validate_safety_flags(bundle["safety_flags"])

    non_claims = _bounded_list(bundle["non_claims"], "non_claims")
    expected_non_claims = [
        "not_model_registry",
        "not_model_promotion_gate",
        "not_live_capture",
        "not_external_enrichment",
        "not_rule_deployment",
        "not_native_runtime_execution",
    ]
    if non_claims != expected_non_claims:
        raise ValueError("non_claims must match the v0 non-claim list")


def _source_schema(source: SourcePayload) -> str:
    if isinstance(source, Mapping):
        schema = source.get("schema_version")
        if schema not in SUPPORTED_REPORT_SCHEMAS:
            raise ValueError(f"unknown report schema_version '{schema}'")
        _validate_safe_tree(source, "bundle source")
        _validate_report_source(source, str(schema))
        return str(schema)

    if isinstance(source, Sequence) and not isinstance(source, str | bytes | bytearray):
        _validate_safe_tree(source, "bundle source")
        _validate_score_rows(source)
        return SCORE_ROWS_SCHEMA_VERSION

    raise ValueError("bundle source must be a report object or score-row list")


def _validate_report_source(report: Mapping[str, Any], schema: str) -> None:
    if schema == DISAGREEMENT_SCHEMA_VERSION:
        _validate_disagreement_report(report)
    elif schema in time_series_residual.SUPPORTED_REPORT_SCHEMA_VERSIONS:
        _validate_residual_report(report)
    elif schema == time_series_forecast_evaluation.REPORT_SCHEMA_VERSION:
        time_series_forecast_evaluation.validate_forecast_evaluation(report)
    elif schema == self_supervised_representation.REPORT_SCHEMA_VERSION:
        _validate_representation_report(report)
    elif schema == temporal_security_graph.REPORT_SCHEMA_VERSION:
        _validate_graph_report(report)
    elif schema == agentic_layer.REPORT_SCHEMA_VERSION:
        agentic_layer.validate_investigation_report(report)
    elif schema == detection_candidates.REPORT_SCHEMA_VERSION:
        detection_candidates.validate_candidate_report(report)
    else:
        raise ValueError(f"unknown report schema_version '{schema}'")


def _validate_disagreement_report(report: Mapping[str, Any]) -> None:
    _require_exact_fields(report, DISAGREEMENT_REPORT_FIELDS, "model disagreement report")
    if report["schema_version"] != DISAGREEMENT_SCHEMA_VERSION:
        raise ValueError("model disagreement report schema_version is invalid")
    _bounded_number(report["model_agreement_score"], "model_agreement_score", 0.0, 1.0)
    _bounded_number(report["model_disagreement_score"], "model_disagreement_score", 0.0, 1.0)
    _bounded_number(report["consensus_risk"], "consensus_risk", 0.0, 1.0)

    row_reports = _bounded_list(report["row_reports"], "row_reports")
    for row in row_reports:
        _validate_disagreement_row(row)

    matrix = _bounded_list(report["model_score_matrix"], "model_score_matrix")
    if len(matrix) != len(row_reports):
        raise ValueError("model_score_matrix length must match row_reports length")

    for field in ("top_supporting_models", "top_dissenting_models"):
        for item in _bounded_list(report[field], field):
            if not isinstance(item, Mapping):
                raise ValueError(f"{field} entries must be objects")
            _required_model_id(item.get("model_id"), "model_id")

    outlier_model = report["outlier_model"]
    if outlier_model is not None:
        _required_model_id(outlier_model, "outlier_model")
    for model_id in _bounded_list(report["outlier_models"], "outlier_models"):
        _required_model_id(model_id, "outlier_models")

    if not isinstance(report["evidence_by_model"], Mapping):
        raise ValueError("evidence_by_model must be an object")


def _validate_disagreement_row(row: Any) -> None:
    if not isinstance(row, Mapping):
        raise ValueError("row_reports entries must be objects")
    _require_exact_fields(row, DISAGREEMENT_ROW_FIELDS, "model disagreement row")
    if row["schema_version"] != ROW_SCHEMA_VERSION:
        raise ValueError(f"row_reports entries require schema_version '{ROW_SCHEMA_VERSION}'")
    _required_entity_id(row["entity_id"], "entity_id")
    _required_window_start(row["window_start"], "window_start")
    _bounded_number(row["agreement_score"], "agreement_score", 0.0, 1.0)
    _bounded_number(row["disagreement_score"], "disagreement_score", 0.0, 1.0)
    _bounded_number(row["consensus_risk"], "consensus_risk", 0.0, 1.0)
    _validated_scores(row["scores"])
    _validated_model_evidence(row["evidence_by_model"], row["scores"])

    outlier_model = row["outlier_model"]
    if outlier_model is not None:
        _required_model_id(outlier_model, "outlier_model")
    for model_id in _bounded_list(row["outlier_models"], "outlier_models"):
        _required_model_id(model_id, "outlier_models")


def _validate_residual_report(report: Mapping[str, Any]) -> None:
    time_series_residual.validate_residual_report(report)


def _validate_representation_report(report: Mapping[str, Any]) -> None:
    _require_exact_fields(report, REPRESENTATION_REPORT_FIELDS, "representation report")
    if report["schema_version"] != self_supervised_representation.REPORT_SCHEMA_VERSION:
        raise ValueError("representation report schema_version is invalid")
    _required_model_id(report["model_id"], "model_id")
    _positive_int(report["embedding_dimensions"], "embedding_dimensions")
    _non_negative_int(report["sequence_count"], "sequence_count")
    _bounded_list(report["token_field_order"], "token_field_order")
    for row in _bounded_list(report["rows"], "rows"):
        self_supervised_representation.validate_representation_evidence_row(row)


def _validate_graph_report(report: Mapping[str, Any]) -> None:
    _require_exact_fields(report, GRAPH_REPORT_FIELDS, "temporal graph report")
    if report["schema_version"] != temporal_security_graph.REPORT_SCHEMA_VERSION:
        raise ValueError("temporal graph report schema_version is invalid")
    _required_model_id(report["model_id"], "model_id")
    _positive_int(report["history_window"], "history_window")
    _positive_int(report["min_history_windows"], "min_history_windows")
    _non_negative_int(report["edge_observation_count"], "edge_observation_count")
    for row in _bounded_list(report["rows"], "rows"):
        temporal_security_graph.validate_temporal_graph_evidence_row(row)


def _validate_score_rows(rows: Sequence[Mapping[str, Any]]) -> None:
    for index, row in enumerate(rows):
        if not isinstance(row, Mapping):
            raise ValueError(f"score row {index} must be an object")
        _require_exact_fields(row, SCORE_ROW_FIELDS, "score row")
        if row["schema_version"] != ROW_SCHEMA_VERSION:
            raise ValueError(f"score row {index} requires schema_version '{ROW_SCHEMA_VERSION}'")
        _required_entity_id(row["entity_id"], "entity_id")
        _required_window_start(row["window_start"], "window_start")
        _validated_score_entries(row["scores"])


def _validated_score_entries(raw_scores: Any) -> dict[str, float]:
    if not isinstance(raw_scores, Mapping) or not raw_scores:
        raise ValueError("score row requires a non-empty scores object")

    scores: dict[str, float] = {}
    for raw_model_id, entry in sorted(raw_scores.items()):
        model_id = _required_model_id(raw_model_id, "model_id")
        if isinstance(entry, int | float):
            scores[model_id] = _bounded_number(entry, model_id, 0.0, 1.0)
            continue
        if not isinstance(entry, Mapping):
            raise ValueError(f"{model_id}: score entry must be a number or object")
        raw_score = entry.get("risk", entry.get("score"))
        if raw_score is None:
            raise ValueError(f"{model_id}: score entry requires risk or score")
        scale = entry.get("scale", "risk")
        if scale == "percentile":
            _bounded_number(raw_score, f"{model_id} percentile", 0.0, 100.0)
        elif scale in {"risk", "anomaly", "confidence", "inverted_risk"}:
            _bounded_number(raw_score, f"{model_id} risk", 0.0, 1.0)
        else:
            raise ValueError(f"{model_id}: unsupported score scale '{scale}'")
    return scores


def _validated_scores(raw_scores: Any) -> dict[str, float]:
    if not isinstance(raw_scores, Mapping) or not raw_scores:
        raise ValueError("scores must be a non-empty object")
    return {
        _required_model_id(model_id, "model_id"): _bounded_number(
            score, f"{model_id} risk", 0.0, 1.0
        )
        for model_id, score in sorted(raw_scores.items())
    }


def _validated_model_evidence(raw_evidence: Any, scores: Mapping[str, Any]) -> None:
    if not isinstance(raw_evidence, Mapping):
        raise ValueError("evidence_by_model must be an object")
    for raw_model_id, entries in sorted(raw_evidence.items()):
        model_id = _required_model_id(raw_model_id, "evidence model_id")
        if model_id not in scores:
            raise ValueError("evidence_by_model model_id must reference scores")
        _bounded_list(entries, f"evidence_by_model[{model_id}]")


def _summarize_source(source: SourcePayload, *, schema: str, source_name: str) -> JsonMap:
    stats = _source_stats()

    if schema == DISAGREEMENT_SCHEMA_VERSION:
        _collect_disagreement_stats(source, stats)
    elif schema in time_series_residual.SUPPORTED_REPORT_SCHEMA_VERSIONS:
        _collect_evidence_report_stats(source, stats, risk_feature_field="feature_name")
    elif schema == time_series_forecast_evaluation.REPORT_SCHEMA_VERSION:
        pass
    elif schema == self_supervised_representation.REPORT_SCHEMA_VERSION:
        _collect_evidence_report_stats(source, stats, sequence_field="sequence_id")
        stats["sequence_ids"].update(
            row["sequence_id"] for row in _report_rows(source) if "sequence_id" in row
        )
        stats["sequence_count_hint"] = _non_negative_int(
            source.get("sequence_count", 0), "sequence_count"
        )
    elif schema == temporal_security_graph.REPORT_SCHEMA_VERSION:
        _collect_evidence_report_stats(source, stats)
        stats["edge_observation_count"] = _non_negative_int(
            source.get("edge_observation_count", 0), "edge_observation_count"
        )
    elif schema == agentic_layer.REPORT_SCHEMA_VERSION:
        _collect_ref_report_stats(source, stats, row_ref_fields=("supporting_evidence_refs",))
        _collect_ref_report_stats(source, stats, row_ref_fields=("refuting_evidence_refs",))
        stats["hypothesis_count"] = len(_report_rows(source))
    elif schema == detection_candidates.REPORT_SCHEMA_VERSION:
        _collect_ref_report_stats(source, stats, row_ref_fields=("source_evidence_refs",))
        rows = _report_rows(source)
        stats["candidate_count"] = len(rows)
        stats["candidate_languages"].update(row["candidate_language"] for row in rows)
        stats["candidate_kinds"].update(row["candidate_kind"] for row in rows)
    elif schema == SCORE_ROWS_SCHEMA_VERSION:
        _collect_score_row_stats(source, stats)
    else:
        raise ValueError(f"unsupported source schema '{schema}'")

    sequence_count = max(len(stats["sequence_ids"]), stats["sequence_count_hint"])
    return {
        "source_name": source_name,
        "source_schema": schema,
        "row_count": stats["row_count"],
        "score_row_count": stats["score_row_count"],
        "entity_count": len(stats["entities"]),
        "window_count": len(stats["windows"]),
        "model_ids": sorted(stats["models"]),
        "feature_count": len(stats["features"]),
        "sequence_count": sequence_count,
        "edge_observation_count": stats["edge_observation_count"],
        "hypothesis_count": stats["hypothesis_count"],
        "candidate_count": stats["candidate_count"],
        "candidate_languages": sorted(stats["candidate_languages"]),
        "candidate_kinds": sorted(stats["candidate_kinds"]),
    }


def _source_stats() -> JsonMap:
    return {
        "row_count": 0,
        "score_row_count": 0,
        "entities": set(),
        "windows": set(),
        "models": set(),
        "features": set(),
        "sequence_ids": set(),
        "sequence_count_hint": 0,
        "edge_observation_count": 0,
        "hypothesis_count": 0,
        "candidate_count": 0,
        "candidate_languages": set(),
        "candidate_kinds": set(),
    }


def _collect_disagreement_stats(source: SourcePayload, stats: JsonMap) -> None:
    if not isinstance(source, Mapping):
        raise ValueError("model disagreement source must be a report object")
    rows = _bounded_list(source["row_reports"], "row_reports")
    stats["row_count"] = len(rows)
    stats["score_row_count"] = len(rows)
    for row in rows:
        _collect_entity_window(row, stats)
        stats["models"].update(row["scores"])


def _collect_evidence_report_stats(
    source: SourcePayload,
    stats: JsonMap,
    *,
    risk_feature_field: str | None = None,
    sequence_field: str | None = None,
) -> None:
    if not isinstance(source, Mapping):
        raise ValueError("evidence source must be a report object")
    rows = _report_rows(source)
    stats["row_count"] = len(rows)
    model_id = source.get("model_id")
    if isinstance(model_id, str):
        stats["models"].add(model_id)
    for row in rows:
        _collect_entity_window(row, stats)
        row_model_id = row.get("model_id")
        if isinstance(row_model_id, str):
            stats["models"].add(row_model_id)
        if risk_feature_field and isinstance(row.get(risk_feature_field), str):
            stats["features"].add(row[risk_feature_field])
        if sequence_field and isinstance(row.get(sequence_field), str):
            stats["sequence_ids"].add(row[sequence_field])


def _collect_score_row_stats(source: SourcePayload, stats: JsonMap) -> None:
    if not isinstance(source, Sequence) or isinstance(source, str | bytes | bytearray):
        raise ValueError("score-row source must be a list")
    stats["row_count"] = len(source)
    stats["score_row_count"] = len(source)
    for row in source:
        _collect_entity_window(row, stats)
        stats["models"].update(row["scores"])
        for score_entry in row["scores"].values():
            _collect_score_entry_features(score_entry, stats)


def _collect_ref_report_stats(
    source: SourcePayload,
    stats: JsonMap,
    *,
    row_ref_fields: Sequence[str],
) -> None:
    if not isinstance(source, Mapping):
        raise ValueError("reference source must be a report object")
    rows = _report_rows(source)
    stats["row_count"] = len(rows)
    for row in rows:
        for ref_field in row_ref_fields:
            for ref in _bounded_list(row[ref_field], ref_field):
                if not isinstance(ref, Mapping):
                    raise ValueError("evidence refs must be objects")
                stats["entities"].add(ref["entity_id"])
                stats["windows"].add(ref["window_start"])
                stats["models"].add(ref["model_id"])


def _collect_score_entry_features(score_entry: Any, stats: JsonMap) -> None:
    if not isinstance(score_entry, Mapping):
        return
    for evidence in _bounded_list(score_entry.get("evidence", []), "evidence"):
        if not isinstance(evidence, Mapping):
            continue
        for column in _bounded_list(evidence.get("feature_columns", []), "feature_columns"):
            if isinstance(column, str) and SAFE_FEATURE_NAME_RE.fullmatch(column):
                stats["features"].add(column)
        for contribution in _bounded_list(
            evidence.get("feature_contributions", []), "feature_contributions"
        ):
            if isinstance(contribution, Mapping) and isinstance(
                contribution.get("feature_name"), str
            ):
                feature_name = contribution["feature_name"]
                if SAFE_FEATURE_NAME_RE.fullmatch(feature_name):
                    stats["features"].add(feature_name)


def _collect_entity_window(row: Mapping[str, Any], stats: JsonMap) -> None:
    stats["entities"].add(row["entity_id"])
    stats["windows"].add(row["window_start"])


def _aggregate_summaries(summaries: Sequence[Mapping[str, Any]]) -> JsonMap:
    schemas = sorted({summary["source_schema"] for summary in summaries})
    source_count_by_schema = Counter(summary["source_schema"] for summary in summaries)
    row_count_by_schema: Counter[str] = Counter()
    model_ids: set[str] = set()
    candidate_languages: set[str] = set()
    candidate_kinds: set[str] = set()

    aggregate = {
        "source_count": len(summaries),
        "schemas_present": schemas,
        "source_count_by_schema": {
            schema: source_count_by_schema[schema] for schema in sorted(source_count_by_schema)
        },
        "row_count_by_schema": {},
        "score_row_count": 0,
        "entity_count": 0,
        "window_count": 0,
        "model_ids": [],
        "feature_count": 0,
        "sequence_count": 0,
        "edge_observation_count": 0,
        "hypothesis_count": 0,
        "candidate_count": 0,
        "candidate_languages": [],
        "candidate_kinds": [],
    }

    for summary in summaries:
        schema = summary["source_schema"]
        row_count_by_schema[schema] += summary["row_count"]
        aggregate["score_row_count"] += summary["score_row_count"]
        aggregate["entity_count"] += summary["entity_count"]
        aggregate["window_count"] += summary["window_count"]
        aggregate["feature_count"] += summary["feature_count"]
        aggregate["sequence_count"] += summary["sequence_count"]
        aggregate["edge_observation_count"] += summary["edge_observation_count"]
        aggregate["hypothesis_count"] += summary["hypothesis_count"]
        aggregate["candidate_count"] += summary["candidate_count"]
        model_ids.update(summary["model_ids"])
        candidate_languages.update(summary["candidate_languages"])
        candidate_kinds.update(summary["candidate_kinds"])

    aggregate["row_count_by_schema"] = {
        schema: row_count_by_schema[schema] for schema in sorted(row_count_by_schema)
    }
    aggregate["model_ids"] = sorted(model_ids)
    aggregate["candidate_languages"] = sorted(candidate_languages)
    aggregate["candidate_kinds"] = sorted(candidate_kinds)
    return aggregate


def _validate_source_summary(summary: Any) -> None:
    if not isinstance(summary, Mapping):
        raise ValueError("source summary must be an object")
    _require_exact_fields(summary, SOURCE_SUMMARY_FIELDS, "source summary")
    source_name = _required_text(summary["source_name"], "source_name")
    if not SAFE_SOURCE_NAME_RE.fullmatch(source_name):
        raise ValueError("source_name must be generated from schema and occurrence only")
    if summary["source_schema"] not in SUPPORTED_SOURCE_SCHEMAS:
        raise ValueError("source_schema is unsupported")
    for count_field in (
        "row_count",
        "score_row_count",
        "entity_count",
        "window_count",
        "feature_count",
        "sequence_count",
        "edge_observation_count",
        "hypothesis_count",
        "candidate_count",
    ):
        _non_negative_int(summary[count_field], count_field)
    for model_id in _bounded_list(summary["model_ids"], "model_ids"):
        _required_model_id(model_id, "model_ids")
    for language in _bounded_list(summary["candidate_languages"], "candidate_languages"):
        if language not in detection_candidates.CANDIDATE_LANGUAGES:
            raise ValueError("candidate_languages contains unsupported language")
    for kind in _bounded_list(summary["candidate_kinds"], "candidate_kinds"):
        if kind not in detection_candidates.CANDIDATE_KINDS:
            raise ValueError("candidate_kinds contains unsupported kind")


def _validate_aggregate_summary(summary: Any) -> None:
    if not isinstance(summary, Mapping):
        raise ValueError("aggregate_summary must be an object")
    _require_exact_fields(summary, AGGREGATE_SUMMARY_FIELDS, "aggregate summary")
    _positive_int(summary["source_count"], "source_count")
    for schema in _bounded_list(summary["schemas_present"], "schemas_present"):
        if schema not in SUPPORTED_SOURCE_SCHEMAS:
            raise ValueError("schemas_present contains unsupported schema")
    for field in ("source_count_by_schema", "row_count_by_schema"):
        value = summary[field]
        if not isinstance(value, Mapping):
            raise ValueError(f"{field} must be an object")
        for schema, count in value.items():
            if schema not in SUPPORTED_SOURCE_SCHEMAS:
                raise ValueError(f"{field} contains unsupported schema")
            _non_negative_int(count, f"{field}.{schema}")
    for count_field in (
        "score_row_count",
        "entity_count",
        "window_count",
        "feature_count",
        "sequence_count",
        "edge_observation_count",
        "hypothesis_count",
        "candidate_count",
    ):
        _non_negative_int(summary[count_field], count_field)
    for model_id in _bounded_list(summary["model_ids"], "model_ids"):
        _required_model_id(model_id, "model_ids")
    for language in _bounded_list(summary["candidate_languages"], "candidate_languages"):
        if language not in detection_candidates.CANDIDATE_LANGUAGES:
            raise ValueError("candidate_languages contains unsupported language")
    for kind in _bounded_list(summary["candidate_kinds"], "candidate_kinds"):
        if kind not in detection_candidates.CANDIDATE_KINDS:
            raise ValueError("candidate_kinds contains unsupported kind")


def _validate_safety_flags(flags: Any) -> None:
    if not isinstance(flags, Mapping):
        raise ValueError("safety_flags must be an object")
    _require_exact_fields(flags, SAFETY_FLAG_FIELDS, "safety flags")
    expected = {
        "local_only": True,
        "strict_json_loaded": True,
        "input_paths_copied": False,
        "source_filenames_copied": False,
        "raw_identifiers_copied": False,
        "generated_artifact_references_copied": False,
        "secrets_detected": False,
        "report_payload_copied": False,
        "live_capture_used": False,
        "external_services_used": False,
        "deployment_allowed": False,
    }
    if dict(flags) != expected:
        raise ValueError("safety_flags must match the v0 local-only false-claim guard")


def _report_rows(report: SourcePayload) -> list[Mapping[str, Any]]:
    if not isinstance(report, Mapping):
        raise ValueError("report source must be an object")
    rows = _bounded_list(report.get("rows", []), "rows")
    loaded: list[Mapping[str, Any]] = []
    for row in rows:
        if not isinstance(row, Mapping):
            raise ValueError("rows entries must be objects")
        loaded.append(row)
    return loaded


def _source_name(schema: str, occurrence: int) -> str:
    return f"{schema.replace('.', '_')}_{occurrence:03d}"


def _input_file(path: str | Path) -> Path:
    source = Path(path)
    if source.is_dir():
        raise ValueError(f"bundle source path must be a file, not a directory: {source}")
    if not source.exists():
        raise ValueError(f"bundle source path does not exist: {source}")
    return source


def _validated_output_path(path: str | Path, *, repo_root: str | Path | None) -> Path:
    output = Path(path)
    if output.is_dir():
        raise ValueError(f"output path must be a file, not a directory: {output}")

    resolved = output.resolve(strict=False)
    repo = Path(repo_root).resolve() if repo_root is not None else _default_repo_root()
    if _is_relative_to(resolved, repo):
        allowed_roots = (
            repo / "data" / "reports",
            repo / "data" / "registry",
            repo / ".runtime",
            repo / "artifacts",
        )
        if not any(_is_relative_to(resolved, root.resolve(strict=False)) for root in allowed_roots):
            raise ValueError(
                "output path inside the repository must be under data/reports/, "
                "data/registry/, .runtime/, or artifacts/"
            )
    return output


def _default_repo_root() -> Path:
    for parent in Path(__file__).resolve().parents:
        if (parent / ".git").exists() or (parent / "AGENTS.md").exists():
            return parent.resolve()
    return Path.cwd().resolve()


def _is_relative_to(child: Path, parent: Path) -> bool:
    try:
        child.relative_to(parent)
    except ValueError:
        return False
    return True


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
            raise ValueError(f"{label} must be finite")
        return

    raise ValueError(f"{label} contains unsupported value type")


def _validate_key(key: Any, label: str) -> None:
    if not isinstance(key, str) or not key.strip():
        raise ValueError(f"{label} object keys must be non-empty strings")
    if len(key) > MAX_STRING_LENGTH:
        raise ValueError(f"{label}.{key} exceeds maximum string length")
    if SECRET_KEY_RE.search(key):
        raise ValueError(f"{label}.{key} contains a secret-like field name")


def _required_text(value: Any, field: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise ValueError(f"{field} must be a non-empty string")
    cleaned = value.strip()
    if len(cleaned) > MAX_STRING_LENGTH:
        raise ValueError(f"{field} exceeds maximum string length")
    _reject_unsafe_text(cleaned, field)
    return cleaned


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
        raise ValueError(f"{field} contains unsafe raw identifier, secret, path, or artifact text")


def _contains_ip_literal(value: str) -> bool:
    for candidate in re.split(r"[\s,;|/]+", value):
        cleaned = candidate.strip("[](){}<>")
        if not cleaned:
            continue
        try:
            ipaddress.ip_address(cleaned)
        except ValueError:
            continue
        return True
    return False


def _required_entity_id(value: Any, field: str) -> str:
    text = _required_text(value, field)
    if not SAFE_ENTITY_ID_RE.fullmatch(text):
        raise ValueError(f"{field} must be a synthetic/coarse entity identifier")
    return text


def _required_model_id(value: Any, field: str) -> str:
    text = _required_text(value, field)
    if not SAFE_MODEL_ID_RE.fullmatch(text):
        raise ValueError(f"{field} must be a sanitized model identifier")
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


def _bounded_list(value: Any, field: str) -> list[Any]:
    if value is None:
        return []
    if not isinstance(value, list):
        raise ValueError(f"{field} must be a list")
    if len(value) > MAX_LIST_LENGTH:
        raise ValueError(f"{field} has too many entries")
    return value


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


def _non_negative_int(raw_value: Any, field: str) -> int:
    if isinstance(raw_value, bool) or not isinstance(raw_value, int):
        raise ValueError(f"{field} must be a non-negative integer")
    if raw_value < 0:
        raise ValueError(f"{field} must be a non-negative integer")
    return raw_value


def _positive_int(raw_value: Any, field: str) -> int:
    value = _non_negative_int(raw_value, field)
    if value < 1:
        raise ValueError(f"{field} must be positive")
    return value


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Generate a local aggregate-only model evaluation bundle."
    )
    parser.add_argument("output", help="Path to write model_evaluation_bundle.v0 JSON")
    parser.add_argument("inputs", nargs="+", help="Local JSON reports or JSON score-row lists")
    args = parser.parse_args(argv)

    bundle = generate_evaluation_bundle(load_bundle_sources(args.inputs))
    dump_bundle(bundle, args.output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
