"""Pointer-only local evidence index over synthetic AI-NDR reports.

The v0 index accepts caller-provided local JSON reports and score-row lists,
then emits only generated source names and row pointers. It does not copy raw
evidence payloads, source paths, filenames, private telemetry, generated
artifact references, capture claims, deployment claims, or external-service
claims.
"""

from __future__ import annotations

import argparse
import json
from collections import Counter
from collections.abc import Mapping, Sequence
from pathlib import Path
from typing import Any

from ares_netguard.detection_engineering import candidates as detection_candidates
from ares_netguard.features import evidence_windows
from ares_netguard.graph import temporal_security_graph
from ares_netguard.investigation import agentic_layer
from ares_netguard.models import (
    evaluation_bundle,
    registry_metadata,
    self_supervised_representation,
    time_series_forecast_evaluation,
    time_series_residual,
)
from ares_netguard.models.disagreement import REPORT_SCHEMA_VERSION as DISAGREEMENT_SCHEMA_VERSION

EVIDENCE_INDEX_SCHEMA_VERSION = "evidence_index.v0"
INDEX_SCOPE = "local_synthetic_evidence_pointer_index"
SCORE_ROWS_SCHEMA_VERSION = evaluation_bundle.SCORE_ROWS_SCHEMA_VERSION

SUPPORTED_REPORT_SCHEMAS = frozenset(
    {
        evidence_windows.REPORT_SCHEMA_VERSION,
        DISAGREEMENT_SCHEMA_VERSION,
        *time_series_residual.SUPPORTED_REPORT_SCHEMA_VERSIONS,
        *time_series_forecast_evaluation.SUPPORTED_REPORT_SCHEMA_VERSIONS,
        self_supervised_representation.REPORT_SCHEMA_VERSION,
        temporal_security_graph.REPORT_SCHEMA_VERSION,
        agentic_layer.REPORT_SCHEMA_VERSION,
        detection_candidates.REPORT_SCHEMA_VERSION,
        registry_metadata.REPORT_SCHEMA_VERSION,
    }
)
SUPPORTED_SOURCE_SCHEMAS = SUPPORTED_REPORT_SCHEMAS | {SCORE_ROWS_SCHEMA_VERSION}

INDEX_FIELDS = frozenset(
    {
        "schema_version",
        "index_scope",
        "source_summaries",
        "entity_window_index",
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
        "entity_window_count",
        "source_ref_count",
        "evidence_ref_count",
        "feature_count",
        "model_count",
        "feature_names",
        "model_ids",
    }
)
ENTITY_WINDOW_FIELDS = frozenset(
    {
        "entity_id",
        "window_start",
        "source_refs",
        "feature_names",
        "model_ids",
        "source_ref_count",
        "evidence_ref_count",
    }
)
SOURCE_REF_FIELDS = frozenset(
    {
        "source_name",
        "source_schema",
        "row_index",
        "row_kind",
        "feature_names",
        "model_ids",
        "evidence_indexes",
    }
)
EVIDENCE_INDEX_REF_FIELDS = frozenset({"model_id", "evidence_index"})
AGGREGATE_SUMMARY_FIELDS = frozenset(
    {
        "source_count",
        "schemas_present",
        "source_count_by_schema",
        "row_count_by_schema",
        "entity_count",
        "entity_window_count",
        "source_ref_count",
        "evidence_ref_count",
        "feature_count",
        "model_count",
        "feature_names",
        "model_ids",
    }
)
SAFETY_FLAG_FIELDS = frozenset(
    {
        "local_only",
        "strict_json_loaded",
        "pointer_only",
        "input_paths_copied",
        "source_filenames_copied",
        "raw_evidence_payload_copied",
        "raw_identifiers_copied",
        "generated_artifact_references_copied",
        "secrets_detected",
        "capture_claims_copied",
        "live_capture_used",
        "external_service_claims_copied",
        "external_services_used",
        "deployment_allowed",
    }
)

NON_CLAIMS = [
    "not_durable_evidence_store",
    "not_database",
    "not_live_capture",
    "not_pcap_parser",
    "not_private_telemetry",
    "not_external_enrichment",
    "not_rule_deployment",
    "not_model_promotion_gate",
    "not_native_runtime_execution",
    "not_qt_binding",
]

FEATURE_METRIC_FIELDS_BY_SCHEMA = {
    self_supervised_representation.REPORT_SCHEMA_VERSION: (
        "embedding_novelty_score",
        "rare_token_count",
        "representation_risk",
    ),
    temporal_security_graph.REPORT_SCHEMA_VERSION: (
        "rare_edge_score",
        "new_neighbor_ratio",
        "degree_change_score",
        "graph_novelty_risk",
    ),
}

JsonMap = dict[str, Any]
SourcePayload = Mapping[str, Any] | Sequence[Mapping[str, Any]]


def load_evidence_source(path: str | Path) -> SourcePayload:
    """Load one strict local JSON report or JSON score-row list."""
    source = _input_file(path)
    payload = evaluation_bundle._loads_strict(source.read_text(encoding="utf-8"))
    if isinstance(payload, Mapping):
        loaded = dict(payload)
        _validate_source_payload(loaded)
        return loaded
    if isinstance(payload, list):
        rows: list[JsonMap] = []
        for index, row in enumerate(payload):
            if not isinstance(row, Mapping):
                raise ValueError(f"score row list entry {index} must be an object")
            rows.append(dict(row))
        _validate_source_payload(rows)
        return rows
    raise ValueError(f"evidence source must be a JSON object or list: {source}")


def load_evidence_sources(paths: Sequence[str | Path]) -> list[SourcePayload]:
    """Load local JSON reports and JSON score-row lists for index generation."""
    return [load_evidence_source(path) for path in paths]


def generate_evidence_index(sources: Sequence[SourcePayload]) -> JsonMap:
    """Generate a deterministic pointer-only evidence index."""
    if not sources:
        raise ValueError("at least one evidence source is required")

    source_summaries: list[JsonMap] = []
    source_refs_by_window: dict[tuple[str, str], list[JsonMap]] = {}
    schema_counts: Counter[str] = Counter()

    for source in sources:
        _validate_source_payload(source)
        schema = _source_schema(source)
        schema_counts[schema] += 1
        source_name = _source_name(schema, schema_counts[schema])
        refs = _source_refs(source, schema=schema, source_name=source_name)
        summary = _source_summary(source, schema=schema, source_name=source_name, refs=refs)
        source_summaries.append(summary)
        for ref in refs:
            key = (str(ref.pop("_entity_id")), str(ref.pop("_window_start")))
            source_refs_by_window.setdefault(key, []).append(ref)

    source_summaries.sort(key=lambda summary: str(summary["source_name"]))
    entity_window_index = _entity_window_rows(source_refs_by_window)
    index = {
        "schema_version": EVIDENCE_INDEX_SCHEMA_VERSION,
        "index_scope": INDEX_SCOPE,
        "source_summaries": source_summaries,
        "entity_window_index": entity_window_index,
        "aggregate_summary": _aggregate_summary(source_summaries, entity_window_index),
        "safety_flags": {
            "local_only": True,
            "strict_json_loaded": True,
            "pointer_only": True,
            "input_paths_copied": False,
            "source_filenames_copied": False,
            "raw_evidence_payload_copied": False,
            "raw_identifiers_copied": False,
            "generated_artifact_references_copied": False,
            "secrets_detected": False,
            "capture_claims_copied": False,
            "live_capture_used": False,
            "external_service_claims_copied": False,
            "external_services_used": False,
            "deployment_allowed": False,
        },
        "non_claims": list(NON_CLAIMS),
    }
    validate_evidence_index(index)
    return index


def dump_evidence_index(
    index: Mapping[str, Any],
    path: str | Path,
    *,
    repo_root: str | Path | None = None,
) -> None:
    """Write a validated evidence index to a non-committed output location."""
    output = evaluation_bundle._validated_output_path(path, repo_root=repo_root)
    validate_evidence_index(index)
    output.write_text(
        json.dumps(index, allow_nan=False, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def validate_evidence_index(index: Mapping[str, Any]) -> None:
    """Validate the strict ``evidence_index.v0`` output contract."""
    if not isinstance(index, Mapping):
        raise ValueError("evidence index must be an object")
    evaluation_bundle._require_exact_fields(index, INDEX_FIELDS, "evidence index")
    evaluation_bundle._validate_safe_tree(index, "evidence index")

    if index["schema_version"] != EVIDENCE_INDEX_SCHEMA_VERSION:
        raise ValueError(
            f"evidence index requires schema_version '{EVIDENCE_INDEX_SCHEMA_VERSION}'"
        )
    if index["index_scope"] != INDEX_SCOPE:
        raise ValueError(f"index_scope must be {INDEX_SCOPE}")

    source_summaries = evaluation_bundle._bounded_list(
        index["source_summaries"], "source_summaries"
    )
    if not source_summaries:
        raise ValueError("source_summaries must not be empty")
    for summary in source_summaries:
        _validate_source_summary(summary)
    source_names = [str(summary["source_name"]) for summary in source_summaries]
    if len(source_names) != len(set(source_names)):
        raise ValueError("source_summaries must have unique source_name values")
    if source_names != sorted(source_names):
        raise ValueError("source_summaries must be sorted by source_name")
    summaries_by_name = {str(summary["source_name"]): summary for summary in source_summaries}
    source_stats = {
        source_name: {
            "entity_windows": set(),
            "source_ref_count": 0,
            "evidence_ref_count": 0,
            "feature_names": set(),
            "model_ids": set(),
        }
        for source_name in source_names
    }

    entity_window_index = evaluation_bundle._bounded_list(
        index["entity_window_index"], "entity_window_index"
    )
    previous_key: tuple[str, str] | None = None
    for row in entity_window_index:
        current_key = _validate_entity_window_row(row, summaries_by_name, source_stats)
        if previous_key is not None and current_key <= previous_key:
            raise ValueError("entity_window_index must be sorted by entity_id and window_start")
        previous_key = current_key
    _validate_source_summaries_match_entity_windows(source_summaries, source_stats)

    _validate_aggregate_summary(index["aggregate_summary"])
    if index["aggregate_summary"] != _aggregate_summary(source_summaries, entity_window_index):
        raise ValueError("aggregate_summary must be derived from source_summaries and index rows")
    _validate_safety_flags(index["safety_flags"])
    if evaluation_bundle._bounded_list(index["non_claims"], "non_claims") != NON_CLAIMS:
        raise ValueError("non_claims must match the v0 non-claim list")


def _validate_source_payload(source: SourcePayload) -> None:
    schema = _source_schema(source)
    if schema == evidence_windows.REPORT_SCHEMA_VERSION:
        if not isinstance(source, Mapping):
            raise ValueError("telemetry feature source must be a report object")
        evidence_windows.validate_feature_window_report(source)
    elif schema in evaluation_bundle.SUPPORTED_REPORT_SCHEMAS:
        if not isinstance(source, Mapping):
            raise ValueError("model report source must be a report object")
        evaluation_bundle._validate_report_source(source, schema)
    elif schema == registry_metadata.REPORT_SCHEMA_VERSION:
        if not isinstance(source, Mapping):
            raise ValueError("registry metadata source must be a report object")
        registry_metadata.validate_registry_metadata(source)
    elif schema == SCORE_ROWS_SCHEMA_VERSION:
        if not isinstance(source, Sequence) or isinstance(source, str | bytes | bytearray):
            raise ValueError("score-row source must be a JSON list")
        evaluation_bundle._validate_score_rows(source)
    else:
        raise ValueError(f"unknown source schema '{schema}'")
    evaluation_bundle._validate_safe_tree(source, "evidence source")


def _source_schema(source: SourcePayload) -> str:
    if isinstance(source, Mapping):
        schema = source.get("schema_version")
        if schema not in SUPPORTED_REPORT_SCHEMAS:
            raise ValueError(f"unknown report schema_version '{schema}'")
        return str(schema)

    if isinstance(source, Sequence) and not isinstance(source, str | bytes | bytearray):
        return SCORE_ROWS_SCHEMA_VERSION

    raise ValueError("evidence source must be a report object or score-row list")


def _source_refs(source: SourcePayload, *, schema: str, source_name: str) -> list[JsonMap]:
    if schema == evidence_windows.REPORT_SCHEMA_VERSION:
        return _telemetry_refs(source, source_name=source_name)
    if schema == DISAGREEMENT_SCHEMA_VERSION:
        return _disagreement_refs(source, source_name=source_name)
    if schema in time_series_residual.SUPPORTED_REPORT_SCHEMA_VERSIONS:
        return _evidence_report_refs(
            source,
            source_name=source_name,
            row_kind="time_series_residual_evidence",
            feature_fields=("feature_name",),
        )
    if schema in time_series_forecast_evaluation.SUPPORTED_REPORT_SCHEMA_VERSIONS:
        return []
    if schema == self_supervised_representation.REPORT_SCHEMA_VERSION:
        return _evidence_report_refs(
            source,
            source_name=source_name,
            row_kind="traffic_representation_evidence",
            metric_feature_names=FEATURE_METRIC_FIELDS_BY_SCHEMA[schema],
        )
    if schema == temporal_security_graph.REPORT_SCHEMA_VERSION:
        return _evidence_report_refs(
            source,
            source_name=source_name,
            row_kind="temporal_security_graph_evidence",
            metric_feature_names=FEATURE_METRIC_FIELDS_BY_SCHEMA[schema],
        )
    if schema == agentic_layer.REPORT_SCHEMA_VERSION:
        return _reference_report_refs(
            source,
            source_name=source_name,
            row_kind="agentic_investigation_hypothesis",
            ref_fields=("supporting_evidence_refs", "refuting_evidence_refs"),
        )
    if schema == detection_candidates.REPORT_SCHEMA_VERSION:
        return _reference_report_refs(
            source,
            source_name=source_name,
            row_kind="detection_candidate",
            ref_fields=("source_evidence_refs",),
        )
    if schema == registry_metadata.REPORT_SCHEMA_VERSION:
        return []
    if schema == SCORE_ROWS_SCHEMA_VERSION:
        return _score_row_refs(source, source_name=source_name)
    raise ValueError(f"unsupported source schema '{schema}'")


def _telemetry_refs(source: SourcePayload, *, source_name: str) -> list[JsonMap]:
    report = _report_object(source, evidence_windows.REPORT_SCHEMA_VERSION)
    refs = []
    for row_index, row in enumerate(_report_rows(report)):
        features = row["features"]
        refs.append(
            _source_ref(
                source_name=source_name,
                source_schema=evidence_windows.REPORT_SCHEMA_VERSION,
                row_index=row_index,
                row_kind="telemetry_feature_window",
                entity_id=str(row["entity_id"]),
                window_start=str(row["window_start"]),
                feature_names=sorted(str(name) for name in features),
            )
        )
    return refs


def _disagreement_refs(source: SourcePayload, *, source_name: str) -> list[JsonMap]:
    report = _report_object(source, DISAGREEMENT_SCHEMA_VERSION)
    refs = []
    for row_index, row in enumerate(_bounded_mapping_rows(report["row_reports"], "row_reports")):
        models = sorted(str(model_id) for model_id in row["scores"])
        refs.append(
            _source_ref(
                source_name=source_name,
                source_schema=DISAGREEMENT_SCHEMA_VERSION,
                row_index=row_index,
                row_kind="model_disagreement_row",
                entity_id=str(row["entity_id"]),
                window_start=str(row["window_start"]),
                model_ids=models,
                evidence_indexes=_model_evidence_indexes(row.get("evidence_by_model", {})),
            )
        )
    return refs


def _evidence_report_refs(
    source: SourcePayload,
    *,
    source_name: str,
    row_kind: str,
    feature_fields: Sequence[str] = (),
    metric_feature_names: Sequence[str] = (),
) -> list[JsonMap]:
    report = _report_object(source, _source_schema(source))
    refs = []
    for row_index, row in enumerate(_report_rows(report)):
        model_id = str(row.get("model_id", report.get("model_id")))
        feature_names = sorted(
            {
                str(row[field])
                for field in feature_fields
                if isinstance(row.get(field), str) and row[field]
            }
            | set(metric_feature_names)
        )
        refs.append(
            _source_ref(
                source_name=source_name,
                source_schema=str(report["schema_version"]),
                row_index=row_index,
                row_kind=row_kind,
                entity_id=str(row["entity_id"]),
                window_start=str(row["window_start"]),
                feature_names=feature_names,
                model_ids=[model_id],
            )
        )
    return refs


def _reference_report_refs(
    source: SourcePayload,
    *,
    source_name: str,
    row_kind: str,
    ref_fields: Sequence[str],
) -> list[JsonMap]:
    report = _report_object(source, _source_schema(source))
    refs = []
    for row_index, row in enumerate(_report_rows(report)):
        refs_by_window: dict[tuple[str, str], set[str]] = {}
        evidence_by_window: dict[tuple[str, str], list[JsonMap]] = {}
        for ref_field in ref_fields:
            for evidence_ref in _bounded_mapping_rows(row[ref_field], ref_field):
                key = (str(evidence_ref["entity_id"]), str(evidence_ref["window_start"]))
                refs_by_window.setdefault(key, set()).add(str(evidence_ref["model_id"]))
                if "evidence_index" in evidence_ref:
                    evidence_by_window.setdefault(key, []).append(
                        {
                            "model_id": str(evidence_ref["model_id"]),
                            "evidence_index": evidence_ref["evidence_index"],
                        }
                    )
        for key in sorted(refs_by_window):
            refs.append(
                _source_ref(
                    source_name=source_name,
                    source_schema=str(report["schema_version"]),
                    row_index=row_index,
                    row_kind=row_kind,
                    entity_id=key[0],
                    window_start=key[1],
                    model_ids=sorted(refs_by_window[key]),
                    evidence_indexes=evidence_by_window.get(key, []),
                )
            )
    return refs


def _score_row_refs(source: SourcePayload, *, source_name: str) -> list[JsonMap]:
    if not isinstance(source, Sequence) or isinstance(source, str | bytes | bytearray):
        raise ValueError("score-row source must be a list")
    refs = []
    for row_index, row in enumerate(source):
        scores = row["scores"]
        refs.append(
            _source_ref(
                source_name=source_name,
                source_schema=SCORE_ROWS_SCHEMA_VERSION,
                row_index=row_index,
                row_kind="model_score_row",
                entity_id=str(row["entity_id"]),
                window_start=str(row["window_start"]),
                feature_names=_score_row_feature_names(scores),
                model_ids=sorted(str(model_id) for model_id in scores),
                evidence_indexes=_score_row_evidence_indexes(scores),
            )
        )
    return refs


def _source_ref(
    *,
    source_name: str,
    source_schema: str,
    row_index: int,
    row_kind: str,
    entity_id: str,
    window_start: str,
    feature_names: Sequence[str] = (),
    model_ids: Sequence[str] = (),
    evidence_indexes: Sequence[Mapping[str, Any]] = (),
) -> JsonMap:
    ref = {
        "source_name": source_name,
        "source_schema": source_schema,
        "row_index": row_index,
        "row_kind": row_kind,
        "feature_names": sorted(set(feature_names)),
        "model_ids": sorted(set(model_ids)),
        "evidence_indexes": _normalized_evidence_indexes(evidence_indexes),
        "_entity_id": entity_id,
        "_window_start": window_start,
    }
    _validate_source_ref({key: value for key, value in ref.items() if not key.startswith("_")})
    evaluation_bundle._required_entity_id(entity_id, "entity_id")
    evaluation_bundle._required_window_start(window_start, "window_start")
    return ref


def _source_summary(
    source: SourcePayload,
    *,
    schema: str,
    source_name: str,
    refs: Sequence[Mapping[str, Any]],
) -> JsonMap:
    feature_names = sorted({name for ref in refs for name in ref["feature_names"]})
    model_ids = _source_model_ids(source, schema=schema, refs=refs)
    summary = {
        "source_name": source_name,
        "source_schema": schema,
        "row_count": _source_row_count(source, schema),
        "entity_window_count": len({(ref["_entity_id"], ref["_window_start"]) for ref in refs}),
        "source_ref_count": len(refs),
        "evidence_ref_count": sum(len(ref["evidence_indexes"]) for ref in refs),
        "feature_count": len(feature_names),
        "model_count": len(model_ids),
        "feature_names": feature_names,
        "model_ids": model_ids,
    }
    _validate_source_summary(summary)
    return summary


def _source_model_ids(
    source: SourcePayload,
    *,
    schema: str,
    refs: Sequence[Mapping[str, Any]],
) -> list[str]:
    model_ids = {model_id for ref in refs for model_id in ref["model_ids"]}
    if schema == registry_metadata.REPORT_SCHEMA_VERSION:
        report = _report_object(source, schema)
        model_ids.update(str(entry["model_id"]) for entry in _report_rows(report, field="entries"))
    return sorted(model_ids)


def _source_row_count(source: SourcePayload, schema: str) -> int:
    if schema == DISAGREEMENT_SCHEMA_VERSION:
        report = _report_object(source, schema)
        return len(evaluation_bundle._bounded_list(report["row_reports"], "row_reports"))
    if schema == registry_metadata.REPORT_SCHEMA_VERSION:
        report = _report_object(source, schema)
        return len(evaluation_bundle._bounded_list(report["entries"], "entries"))
    if schema == SCORE_ROWS_SCHEMA_VERSION:
        if not isinstance(source, Sequence) or isinstance(source, str | bytes | bytearray):
            raise ValueError("score-row source must be a list")
        return len(source)
    report = _report_object(source, schema)
    return len(evaluation_bundle._bounded_list(report.get("rows", []), "rows"))


def _entity_window_rows(
    source_refs_by_window: Mapping[tuple[str, str], Sequence[Mapping[str, Any]]],
) -> list[JsonMap]:
    rows = []
    for entity_id, window_start in sorted(source_refs_by_window):
        refs = sorted(
            [dict(ref) for ref in source_refs_by_window[(entity_id, window_start)]],
            key=lambda item: (
                item["source_name"],
                item["source_schema"],
                item["row_index"],
                item["row_kind"],
            ),
        )
        feature_names = sorted({name for ref in refs for name in ref["feature_names"]})
        model_ids = sorted({model_id for ref in refs for model_id in ref["model_ids"]})
        row = {
            "entity_id": entity_id,
            "window_start": window_start,
            "source_refs": refs,
            "feature_names": feature_names,
            "model_ids": model_ids,
            "source_ref_count": len(refs),
            "evidence_ref_count": sum(len(ref["evidence_indexes"]) for ref in refs),
        }
        _validate_entity_window_row(row)
        rows.append(row)
    return rows


def _aggregate_summary(
    source_summaries: Sequence[Mapping[str, Any]],
    entity_window_index: Sequence[Mapping[str, Any]],
) -> JsonMap:
    source_count_by_schema = Counter(str(summary["source_schema"]) for summary in source_summaries)
    row_count_by_schema: Counter[str] = Counter()
    feature_names = {name for row in entity_window_index for name in row["feature_names"]}
    model_ids = {model_id for summary in source_summaries for model_id in summary["model_ids"]}

    for summary in source_summaries:
        row_count_by_schema[str(summary["source_schema"])] += int(summary["row_count"])
        feature_names.update(str(name) for name in summary["feature_names"])

    return {
        "source_count": len(source_summaries),
        "schemas_present": sorted(source_count_by_schema),
        "source_count_by_schema": {
            schema: source_count_by_schema[schema] for schema in sorted(source_count_by_schema)
        },
        "row_count_by_schema": {
            schema: row_count_by_schema[schema] for schema in sorted(row_count_by_schema)
        },
        "entity_count": len({str(row["entity_id"]) for row in entity_window_index}),
        "entity_window_count": len(entity_window_index),
        "source_ref_count": sum(int(row["source_ref_count"]) for row in entity_window_index),
        "evidence_ref_count": sum(int(row["evidence_ref_count"]) for row in entity_window_index),
        "feature_count": len(feature_names),
        "model_count": len(model_ids),
        "feature_names": sorted(feature_names),
        "model_ids": sorted(model_ids),
    }


def _validate_source_summary(summary: Any) -> None:
    if not isinstance(summary, Mapping):
        raise ValueError("source summary must be an object")
    evaluation_bundle._require_exact_fields(summary, SOURCE_SUMMARY_FIELDS, "source summary")
    source_name = evaluation_bundle._required_text(summary["source_name"], "source_name")
    if not evaluation_bundle.SAFE_SOURCE_NAME_RE.fullmatch(source_name):
        raise ValueError("source_name must be generated from schema and occurrence only")
    if summary["source_schema"] not in SUPPORTED_SOURCE_SCHEMAS:
        raise ValueError("source_schema is unsupported")
    for field in (
        "row_count",
        "entity_window_count",
        "source_ref_count",
        "evidence_ref_count",
        "feature_count",
        "model_count",
    ):
        evaluation_bundle._non_negative_int(summary[field], field)
    _validate_feature_names(summary["feature_names"])
    _validate_model_ids(summary["model_ids"])
    if summary["feature_count"] != len(summary["feature_names"]):
        raise ValueError("feature_count must match feature_names")
    if summary["model_count"] != len(summary["model_ids"]):
        raise ValueError("model_count must match model_ids")


def _validate_source_summaries_match_entity_windows(
    source_summaries: Sequence[Mapping[str, Any]],
    source_stats: Mapping[str, Mapping[str, Any]],
) -> None:
    for summary in source_summaries:
        source_name = str(summary["source_name"])
        stats = source_stats[source_name]
        if summary["entity_window_count"] != len(stats["entity_windows"]):
            raise ValueError("source summary entity_window_count must match index rows")
        if summary["source_ref_count"] != stats["source_ref_count"]:
            raise ValueError("source summary source_ref_count must match index rows")
        if summary["evidence_ref_count"] != stats["evidence_ref_count"]:
            raise ValueError("source summary evidence_ref_count must match index rows")
        if summary["feature_names"] != sorted(stats["feature_names"]):
            raise ValueError("source summary feature_names must match index rows")
        if summary["source_schema"] != registry_metadata.REPORT_SCHEMA_VERSION and summary[
            "model_ids"
        ] != sorted(stats["model_ids"]):
            raise ValueError("source summary model_ids must match index rows")


def _validate_entity_window_row(
    row: Any,
    summaries_by_name: Mapping[str, Mapping[str, Any]] | None = None,
    source_stats: Mapping[str, dict[str, Any]] | None = None,
) -> tuple[str, str]:
    if not isinstance(row, Mapping):
        raise ValueError("entity_window_index row must be an object")
    evaluation_bundle._require_exact_fields(row, ENTITY_WINDOW_FIELDS, "entity window row")
    entity_id = evaluation_bundle._required_entity_id(row["entity_id"], "entity_id")
    window_start = evaluation_bundle._required_window_start(row["window_start"], "window_start")
    refs = evaluation_bundle._bounded_list(row["source_refs"], "source_refs")
    if not refs:
        raise ValueError("entity window row must contain at least one source_ref")
    previous_ref: tuple[str, str, int, str] | None = None
    for ref in refs:
        current_ref = _validate_source_ref(ref)
        if previous_ref is not None and current_ref <= previous_ref:
            raise ValueError("source_refs must be sorted and unique")
        previous_ref = current_ref
        if summaries_by_name is not None and source_stats is not None:
            source_name = current_ref[0]
            summary = summaries_by_name.get(source_name)
            if summary is None:
                raise ValueError("source_ref source_name must reference a source summary")
            if summary["source_schema"] != ref["source_schema"]:
                raise ValueError("source_ref source_schema must match source summary")
            if ref["row_index"] >= summary["row_count"]:
                raise ValueError("source_ref row_index must be inside source summary row_count")
            stats = source_stats[source_name]
            stats["entity_windows"].add((entity_id, window_start))
            stats["source_ref_count"] += 1
            stats["evidence_ref_count"] += len(ref["evidence_indexes"])
            stats["feature_names"].update(ref["feature_names"])
            stats["model_ids"].update(ref["model_ids"])
    _validate_feature_names(row["feature_names"])
    _validate_model_ids(row["model_ids"])
    if row["feature_names"] != sorted({name for ref in refs for name in ref["feature_names"]}):
        raise ValueError("entity window feature_names must be derived from source_refs")
    if row["model_ids"] != sorted({model_id for ref in refs for model_id in ref["model_ids"]}):
        raise ValueError("entity window model_ids must be derived from source_refs")
    if row["source_ref_count"] != len(refs):
        raise ValueError("source_ref_count must match source_refs length")
    evidence_ref_count = sum(len(ref["evidence_indexes"]) for ref in refs)
    if row["evidence_ref_count"] != evidence_ref_count:
        raise ValueError("evidence_ref_count must match source_refs")
    return (entity_id, window_start)


def _validate_source_ref(ref: Any) -> tuple[str, str, int, str]:
    if not isinstance(ref, Mapping):
        raise ValueError("source_ref must be an object")
    evaluation_bundle._require_exact_fields(ref, SOURCE_REF_FIELDS, "source_ref")
    source_name = evaluation_bundle._required_text(ref["source_name"], "source_name")
    if not evaluation_bundle.SAFE_SOURCE_NAME_RE.fullmatch(source_name):
        raise ValueError("source_ref source_name must be generated")
    if ref["source_schema"] not in SUPPORTED_SOURCE_SCHEMAS:
        raise ValueError("source_ref source_schema is unsupported")
    row_index = evaluation_bundle._non_negative_int(ref["row_index"], "row_index")
    row_kind = evaluation_bundle._required_text(ref["row_kind"], "row_kind")
    if not evaluation_bundle.SAFE_MODEL_ID_RE.fullmatch(row_kind):
        raise ValueError("row_kind must be a sanitized pointer label")
    _validate_feature_names(ref["feature_names"])
    _validate_model_ids(ref["model_ids"])
    _validate_evidence_indexes(ref["evidence_indexes"])
    model_ids = set(ref["model_ids"])
    for evidence_ref in ref["evidence_indexes"]:
        if evidence_ref["model_id"] not in model_ids:
            raise ValueError("evidence index model_id must be present in source_ref model_ids")
    return (source_name, str(ref["source_schema"]), row_index, row_kind)


def _validate_aggregate_summary(summary: Any) -> None:
    if not isinstance(summary, Mapping):
        raise ValueError("aggregate_summary must be an object")
    evaluation_bundle._require_exact_fields(summary, AGGREGATE_SUMMARY_FIELDS, "aggregate summary")
    evaluation_bundle._positive_int(summary["source_count"], "source_count")
    for schema in evaluation_bundle._bounded_list(summary["schemas_present"], "schemas_present"):
        if schema not in SUPPORTED_SOURCE_SCHEMAS:
            raise ValueError("schemas_present contains unsupported schema")
    for field in ("source_count_by_schema", "row_count_by_schema"):
        value = summary[field]
        if not isinstance(value, Mapping):
            raise ValueError(f"{field} must be an object")
        for schema, count in value.items():
            if schema not in SUPPORTED_SOURCE_SCHEMAS:
                raise ValueError(f"{field} contains unsupported schema")
            evaluation_bundle._non_negative_int(count, f"{field}.{schema}")
    for field in (
        "entity_count",
        "entity_window_count",
        "source_ref_count",
        "evidence_ref_count",
        "feature_count",
        "model_count",
    ):
        evaluation_bundle._non_negative_int(summary[field], field)
    _validate_feature_names(summary["feature_names"])
    _validate_model_ids(summary["model_ids"])
    if summary["feature_count"] != len(summary["feature_names"]):
        raise ValueError("feature_count must match feature_names")
    if summary["model_count"] != len(summary["model_ids"]):
        raise ValueError("model_count must match model_ids")


def _validate_safety_flags(flags: Any) -> None:
    if not isinstance(flags, Mapping):
        raise ValueError("safety_flags must be an object")
    evaluation_bundle._require_exact_fields(flags, SAFETY_FLAG_FIELDS, "safety flags")
    expected = {
        "local_only": True,
        "strict_json_loaded": True,
        "pointer_only": True,
        "input_paths_copied": False,
        "source_filenames_copied": False,
        "raw_evidence_payload_copied": False,
        "raw_identifiers_copied": False,
        "generated_artifact_references_copied": False,
        "secrets_detected": False,
        "capture_claims_copied": False,
        "live_capture_used": False,
        "external_service_claims_copied": False,
        "external_services_used": False,
        "deployment_allowed": False,
    }
    if dict(flags) != expected:
        raise ValueError("safety_flags must match the v0 pointer-only false-claim guard")


def _validate_feature_names(raw_names: Any) -> None:
    names = evaluation_bundle._bounded_list(raw_names, "feature_names")
    if names != sorted(set(names)):
        raise ValueError("feature_names must be sorted and unique")
    for name in names:
        text = evaluation_bundle._required_text(name, "feature_name")
        if not evaluation_bundle.SAFE_FEATURE_NAME_RE.fullmatch(text):
            raise ValueError("feature_name must be sanitized")


def _validate_model_ids(raw_model_ids: Any) -> None:
    model_ids = evaluation_bundle._bounded_list(raw_model_ids, "model_ids")
    if model_ids != sorted(set(model_ids)):
        raise ValueError("model_ids must be sorted and unique")
    for model_id in model_ids:
        evaluation_bundle._required_model_id(model_id, "model_id")


def _validate_evidence_indexes(raw_indexes: Any) -> None:
    refs = evaluation_bundle._bounded_list(raw_indexes, "evidence_indexes")
    previous: tuple[str, int] | None = None
    for ref in refs:
        if not isinstance(ref, Mapping):
            raise ValueError("evidence_indexes entries must be objects")
        evaluation_bundle._require_exact_fields(ref, EVIDENCE_INDEX_REF_FIELDS, "evidence index")
        model_id = evaluation_bundle._required_model_id(ref["model_id"], "model_id")
        evidence_index = evaluation_bundle._non_negative_int(
            ref["evidence_index"], "evidence_index"
        )
        current = (model_id, evidence_index)
        if previous is not None and current <= previous:
            raise ValueError("evidence_indexes must be sorted and unique")
        previous = current


def _report_object(source: SourcePayload, schema: str) -> Mapping[str, Any]:
    if not isinstance(source, Mapping):
        raise ValueError(f"{schema} source must be a report object")
    if source.get("schema_version") != schema:
        raise ValueError(f"source requires schema_version '{schema}'")
    return source


def _report_rows(report: Mapping[str, Any], *, field: str = "rows") -> list[Mapping[str, Any]]:
    return _bounded_mapping_rows(report.get(field, []), field)


def _bounded_mapping_rows(rows: Any, field: str) -> list[Mapping[str, Any]]:
    loaded = []
    for row in evaluation_bundle._bounded_list(rows, field):
        if not isinstance(row, Mapping):
            raise ValueError(f"{field} entries must be objects")
        loaded.append(row)
    return loaded


def _model_evidence_indexes(raw_evidence_by_model: Any) -> list[JsonMap]:
    if not isinstance(raw_evidence_by_model, Mapping):
        return []
    refs = []
    for model_id, entries in sorted(raw_evidence_by_model.items()):
        evaluation_bundle._required_model_id(model_id, "model_id")
        for evidence_index, _entry in enumerate(
            evaluation_bundle._bounded_list(entries, f"evidence_by_model[{model_id}]")
        ):
            refs.append({"model_id": str(model_id), "evidence_index": evidence_index})
    return _normalized_evidence_indexes(refs)


def _score_row_evidence_indexes(raw_scores: Any) -> list[JsonMap]:
    if not isinstance(raw_scores, Mapping):
        return []
    refs = []
    for model_id, score_entry in sorted(raw_scores.items()):
        evaluation_bundle._required_model_id(model_id, "model_id")
        if not isinstance(score_entry, Mapping):
            continue
        for evidence_index, _entry in enumerate(
            evaluation_bundle._bounded_list(score_entry.get("evidence", []), "evidence")
        ):
            refs.append({"model_id": str(model_id), "evidence_index": evidence_index})
    return _normalized_evidence_indexes(refs)


def _score_row_feature_names(raw_scores: Any) -> list[str]:
    feature_names: set[str] = set()
    if not isinstance(raw_scores, Mapping):
        return []
    for score_entry in raw_scores.values():
        if not isinstance(score_entry, Mapping):
            continue
        for evidence in evaluation_bundle._bounded_list(
            score_entry.get("evidence", []), "evidence"
        ):
            if not isinstance(evidence, Mapping):
                continue
            for column in evaluation_bundle._bounded_list(
                evidence.get("feature_columns", []), "feature_columns"
            ):
                if isinstance(column, str) and evaluation_bundle.SAFE_FEATURE_NAME_RE.fullmatch(
                    column
                ):
                    feature_names.add(column)
            for contribution in evaluation_bundle._bounded_list(
                evidence.get("feature_contributions", []), "feature_contributions"
            ):
                if not isinstance(contribution, Mapping):
                    continue
                feature_name = contribution.get("feature_name")
                if isinstance(
                    feature_name, str
                ) and evaluation_bundle.SAFE_FEATURE_NAME_RE.fullmatch(feature_name):
                    feature_names.add(feature_name)
    return sorted(feature_names)


def _normalized_evidence_indexes(raw_indexes: Sequence[Mapping[str, Any]]) -> list[JsonMap]:
    refs = [
        {
            "model_id": evaluation_bundle._required_model_id(ref["model_id"], "model_id"),
            "evidence_index": evaluation_bundle._non_negative_int(
                ref["evidence_index"], "evidence_index"
            ),
        }
        for ref in raw_indexes
    ]
    return sorted(
        {json.dumps(ref, allow_nan=False, sort_keys=True): ref for ref in refs}.values(),
        key=lambda item: (item["model_id"], item["evidence_index"]),
    )


def _source_name(schema: str, occurrence: int) -> str:
    return f"{schema.replace('.', '_')}_{occurrence:03d}"


def _input_file(path: str | Path) -> Path:
    source = Path(path)
    if source.is_dir():
        raise ValueError(f"evidence source path must be a file, not a directory: {source}")
    if not source.exists():
        raise ValueError(f"evidence source path does not exist: {source}")
    return source


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Generate a local pointer-only evidence_index.v0 JSON file."
    )
    parser.add_argument("output", help="Path to write evidence_index.v0 JSON")
    parser.add_argument("inputs", nargs="+", help="Local JSON reports or JSON score-row lists")
    args = parser.parse_args(argv)

    index = generate_evidence_index(load_evidence_sources(args.inputs))
    dump_evidence_index(index, args.output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
