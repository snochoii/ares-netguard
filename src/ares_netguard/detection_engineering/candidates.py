"""Deterministic offline detection engineering candidate generation.

The v0 generator works only over local, already-generated synthetic model
disagreement reports. It emits draft candidate rows for analyst review; it
does not deploy rules, enrich indicators, perform probing, inspect payloads,
or call external services.
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

REPORT_SCHEMA_VERSION = "detection_candidate_report.v0"
SOURCE_REPORT_SCHEMA_VERSION = "model_disagreement_report.v0"
ROW_SCHEMA_VERSION = "model_score_row.v0"

HIGH_CONSENSUS_RISK_THRESHOLD = 0.75
SUPPORTING_MODEL_RISK_THRESHOLD = 0.7
HIGH_DISAGREEMENT_THRESHOLD = 0.5
DRAFT_MARKER = "DRAFT_DO_NOT_DEPLOY"

CANDIDATE_LANGUAGES = ("zeek", "sigma_like", "suricata_local", "siem_query")
HIGH_CONSENSUS_KIND = "high_consensus_risk"
HIGH_DISAGREEMENT_KIND = "high_model_disagreement"
CANDIDATE_KINDS = (HIGH_CONSENSUS_KIND, HIGH_DISAGREEMENT_KIND)

MAX_STRING_LENGTH = 1024
MAX_LIST_LENGTH = 10000
MAX_MAPPING_LENGTH = 128
MAX_DEPTH = 16
MAX_REFS_PER_CANDIDATE = 16

REPORT_FIELDS = frozenset(
    {
        "schema_version",
        "source_report_schema",
        "validation_summary",
        "rows",
    }
)
CANDIDATE_ROW_FIELDS = frozenset(
    {
        "candidate_id",
        "candidate_language",
        "candidate_kind",
        "title",
        "draft",
        "source_evidence_refs",
        "validation",
        "false_positive_estimate",
        "human_review_required",
        "deployment_allowed",
    }
)
VALIDATION_SUMMARY_FIELDS = frozenset(
    {
        "source_rows_considered",
        "eligible_patterns",
        "candidates_generated",
        "candidate_languages",
        "validation_scope",
        "human_review_required",
        "deployment_allowed",
    }
)
ROW_VALIDATION_FIELDS = frozenset(
    {
        "source_row_index",
        "eligible_pattern",
        "fixture_validation",
        "candidate_language_checked",
        "synthetic_source_required",
        "validated_against_replay",
    }
)
FALSE_POSITIVE_FIELDS = frozenset({"label", "basis"})
REF_REQUIRED_FIELDS = frozenset(
    {
        "report_schema",
        "entity_id",
        "window_start",
        "model_id",
        "row_index",
        "field_path",
    }
)
REF_ALLOWED_FIELDS = REF_REQUIRED_FIELDS | {"evidence_index"}

JsonMap = dict[str, Any]

SAFE_ENTITY_ID_RE = re.compile(r"^(?:asset|entity|fixture|host|sensor)-[a-z0-9][a-z0-9_-]{0,62}$")
SAFE_MODEL_ID_RE = re.compile(r"^[a-z][a-z0-9_-]{0,80}$")
SAFE_FIELD_PATH_RE = re.compile(r"^\[[A-Za-z0-9_-]+](?:\[[A-Za-z0-9_-]+])*$")
SAFE_CANDIDATE_ID_RE = re.compile(
    r"^cand-v0-[0-9]{4}-(?:high_consensus_risk|high_model_disagreement)-"
    r"(?:zeek|sigma_like|suricata_local|siem_query)$"
)
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
)


def load_report(path: str | Path) -> JsonMap:
    """Load a model_disagreement_report.v0 JSON file using strict JSON constants."""
    source = Path(path)
    if source.is_dir():
        raise ValueError(f"report path must be a file, not a directory: {source}")

    payload = _loads_strict(source.read_text(encoding="utf-8"))
    if not isinstance(payload, Mapping):
        raise ValueError(f"report payload must be an object: {source}")

    report = dict(payload)
    schema_version = report.get("schema_version")
    if schema_version != SOURCE_REPORT_SCHEMA_VERSION:
        raise ValueError(f"unknown report schema_version '{schema_version}'")
    _validate_safe_tree(report, "model disagreement report")
    return report


def generate_candidate_report(disagreement_report: Mapping[str, Any]) -> JsonMap:
    """Generate deterministic draft detection candidates from a disagreement report."""
    row_entries = _validated_disagreement_rows(disagreement_report)

    candidates: list[JsonMap] = []
    eligible_patterns = 0
    for row_index, row in row_entries:
        patterns = _eligible_patterns(row)
        eligible_patterns += len(patterns)
        for pattern in patterns:
            for language in CANDIDATE_LANGUAGES:
                candidates.append(_candidate(row_index, row, pattern, language))

    report = {
        "schema_version": REPORT_SCHEMA_VERSION,
        "source_report_schema": SOURCE_REPORT_SCHEMA_VERSION,
        "validation_summary": {
            "source_rows_considered": len(row_entries),
            "eligible_patterns": eligible_patterns,
            "candidates_generated": len(candidates),
            "candidate_languages": list(CANDIDATE_LANGUAGES),
            "validation_scope": "source_report_schema_and_privacy_guards_only",
            "human_review_required": True,
            "deployment_allowed": False,
        },
        "rows": candidates,
    }
    validate_candidate_report(report)
    return report


def validate_candidate_report(report: Mapping[str, Any]) -> None:
    """Validate the strict v0 detection candidate report contract."""
    _validate_safe_tree(report, "candidate report")
    _require_exact_fields(report, REPORT_FIELDS, "candidate report")
    if report.get("schema_version") != REPORT_SCHEMA_VERSION:
        raise ValueError(f"candidate report requires schema_version '{REPORT_SCHEMA_VERSION}'")
    if report.get("source_report_schema") != SOURCE_REPORT_SCHEMA_VERSION:
        raise ValueError(f"source_report_schema must be '{SOURCE_REPORT_SCHEMA_VERSION}'")
    _validate_summary(report["validation_summary"])
    for row in _bounded_list(report["rows"], "rows"):
        _validate_candidate_row(row)


def dump_report(report: Mapping[str, Any], path: str | Path) -> None:
    """Write candidate report JSON with stable formatting and strict finite numbers."""
    output = Path(path)
    if output.is_dir():
        raise ValueError(f"output path must be a file, not a directory: {output}")
    validate_candidate_report(report)
    output.write_text(
        json.dumps(report, allow_nan=False, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def _eligible_patterns(row: Mapping[str, Any]) -> list[JsonMap]:
    patterns: list[JsonMap] = []
    supporting_models = _supporting_models(row)
    if row["consensus_risk"] >= HIGH_CONSENSUS_RISK_THRESHOLD and len(supporting_models) >= 2:
        patterns.append(
            {
                "kind": HIGH_CONSENSUS_KIND,
                "supporting_models": supporting_models,
                "outlier_model": None,
            }
        )

    outlier_model = row.get("outlier_model")
    if (
        row["disagreement_score"] >= HIGH_DISAGREEMENT_THRESHOLD
        and isinstance(outlier_model, str)
        and outlier_model
    ):
        patterns.append(
            {
                "kind": HIGH_DISAGREEMENT_KIND,
                "supporting_models": [outlier_model],
                "outlier_model": outlier_model,
            }
        )
    return patterns


def _candidate(
    row_index: int,
    row: Mapping[str, Any],
    pattern: Mapping[str, Any],
    language: str,
) -> JsonMap:
    kind = _candidate_kind(pattern["kind"])
    candidate_id = f"cand-v0-{row_index:04d}-{kind}-{language}"
    title = _title(language, kind, row, pattern)
    return {
        "candidate_id": candidate_id,
        "candidate_language": language,
        "candidate_kind": kind,
        "title": title,
        "draft": _draft(language, kind, row, pattern),
        "source_evidence_refs": _source_refs(row_index, row, pattern),
        "validation": {
            "source_row_index": row_index,
            "eligible_pattern": kind,
            "fixture_validation": "source_report_schema_validated",
            "candidate_language_checked": language,
            "synthetic_source_required": True,
            "validated_against_replay": False,
        },
        "false_positive_estimate": _false_positive_estimate(kind, pattern),
        "human_review_required": True,
        "deployment_allowed": False,
    }


def _title(
    language: str,
    kind: str,
    row: Mapping[str, Any],
    pattern: Mapping[str, Any],
) -> str:
    if kind == HIGH_CONSENSUS_KIND:
        model_count = len(pattern["supporting_models"])
        return (
            f"{language} draft for high consensus risk on {row['entity_id']} "
            f"with {model_count} supporting models"
        )
    return (
        f"{language} draft for model disagreement on {row['entity_id']} "
        f"with outlier {pattern['outlier_model']}"
    )


def _draft(
    language: str,
    kind: str,
    row: Mapping[str, Any],
    pattern: Mapping[str, Any],
) -> str:
    entity_id = row["entity_id"]
    window_start = row["window_start"]
    if kind == HIGH_CONSENSUS_KIND:
        model_phrase = _model_phrase(pattern["supporting_models"])
        if language == "zeek":
            return (
                f"{DRAFT_MARKER} zeek_candidate kind={kind} entity={entity_id} "
                f"window={window_start} supporting_models={model_phrase} "
                f"condition=consensus_risk_at_least_0_75_and_two_models_at_least_0_70"
            )
        if language == "sigma_like":
            return (
                f"{DRAFT_MARKER} sigma_like_candidate kind={kind} entity={entity_id} "
                f"window={window_start} supporting_models={model_phrase} "
                f"condition=consensus_risk_gte_0_75_and_supporting_model_count_gte_2"
            )
        if language == "suricata_local":
            return (
                f"{DRAFT_MARKER} suricata_local_candidate kind={kind} entity={entity_id} "
                f"window={window_start} metadata_models={model_phrase} "
                f"metadata_condition=synthetic_consensus_risk_gte_0_75"
            )
        if language == "siem_query":
            return (
                f"{DRAFT_MARKER} siem_query_candidate source={SOURCE_REPORT_SCHEMA_VERSION} "
                f"kind={kind} entity={entity_id} window={window_start} "
                f"condition=consensus_risk_gte_0_75 supporting_model_count_gte_2"
            )
    else:
        outlier_model = pattern["outlier_model"]
        if language == "zeek":
            return (
                f"{DRAFT_MARKER} zeek_candidate kind={kind} entity={entity_id} "
                f"window={window_start} outlier_model={outlier_model} "
                f"condition=disagreement_score_at_least_0_50"
            )
        if language == "sigma_like":
            return (
                f"{DRAFT_MARKER} sigma_like_candidate kind={kind} entity={entity_id} "
                f"window={window_start} outlier_model={outlier_model} "
                f"condition=disagreement_score_gte_0_50_and_outlier_model_present"
            )
        if language == "suricata_local":
            return (
                f"{DRAFT_MARKER} suricata_local_candidate kind={kind} entity={entity_id} "
                f"window={window_start} metadata_outlier_model={outlier_model} "
                f"metadata_condition=synthetic_disagreement_score_gte_0_50"
            )
        if language == "siem_query":
            return (
                f"{DRAFT_MARKER} siem_query_candidate source={SOURCE_REPORT_SCHEMA_VERSION} "
                f"kind={kind} entity={entity_id} window={window_start} "
                f"condition=disagreement_score_gte_0_50 outlier_model={outlier_model}"
            )
    raise ValueError(f"unsupported candidate_language '{language}'")


def _source_refs(
    row_index: int,
    row: Mapping[str, Any],
    pattern: Mapping[str, Any],
) -> list[JsonMap]:
    kind = _candidate_kind(pattern["kind"])
    if kind == HIGH_CONSENSUS_KIND:
        supporting_models = list(pattern["supporting_models"])
        refs = [
            _row_field_ref(row_index, row, "model_disagreement", "consensus_risk"),
            *[_score_ref(row_index, row, model_id) for model_id in supporting_models],
            *_model_evidence_refs(row_index, row, supporting_models),
        ]
    else:
        outlier_model = _required_model_id(pattern["outlier_model"], "outlier_model")
        refs = [
            _row_field_ref(row_index, row, "model_disagreement", "disagreement_score"),
            _score_ref(row_index, row, outlier_model),
            *_model_evidence_refs(row_index, row, [outlier_model]),
        ]
    return _limited_refs(refs)


def _false_positive_estimate(kind: str, pattern: Mapping[str, Any]) -> JsonMap:
    if kind == HIGH_CONSENSUS_KIND:
        if len(pattern["supporting_models"]) >= 3:
            label = "medium"
            basis = "three_or_more_synthetic_model_signals_without_replay_validation"
        else:
            label = "high"
            basis = "two_synthetic_model_signals_require_fixture_replay_and_analyst_review"
    else:
        label = "high"
        basis = "model_disagreement_is_a_review_signal_not_a_deployable_detection"
    return {"label": label, "basis": basis}


def _validated_disagreement_rows(report: Mapping[str, Any]) -> list[tuple[int, JsonMap]]:
    _report_schema(report, SOURCE_REPORT_SCHEMA_VERSION)
    _validate_safe_tree(report, "model disagreement report")

    rows = _bounded_list(report.get("row_reports"), "row_reports")
    row_entries: list[tuple[int, JsonMap]] = []
    for row_index, raw_row in enumerate(rows):
        if not isinstance(raw_row, Mapping):
            raise ValueError("row_reports entries must be objects")
        if raw_row.get("schema_version") != ROW_SCHEMA_VERSION:
            raise ValueError(f"row_report requires schema_version '{ROW_SCHEMA_VERSION}'")

        entity_id = _required_entity_id(raw_row.get("entity_id"), "entity_id")
        window_start = _required_window_start(raw_row.get("window_start"), "window_start")
        scores = _validated_scores(raw_row.get("scores"))
        evidence_by_model = _validated_model_evidence(raw_row.get("evidence_by_model"), scores)
        outlier_model = raw_row.get("outlier_model")
        if outlier_model is not None:
            outlier_model = _required_model_id(outlier_model, "outlier_model")
            if outlier_model not in scores:
                raise ValueError("outlier_model must reference a model in scores")

        row_entries.append(
            (
                row_index,
                {
                    "entity_id": entity_id,
                    "window_start": window_start,
                    "scores": scores,
                    "agreement_score": _bounded_number(
                        raw_row.get("agreement_score"),
                        "agreement_score",
                        0.0,
                        1.0,
                    ),
                    "disagreement_score": _bounded_number(
                        raw_row.get("disagreement_score"),
                        "disagreement_score",
                        0.0,
                        1.0,
                    ),
                    "consensus_risk": _bounded_number(
                        raw_row.get("consensus_risk"),
                        "consensus_risk",
                        0.0,
                        1.0,
                    ),
                    "outlier_model": outlier_model,
                    "evidence_by_model": evidence_by_model,
                },
            )
        )
    return row_entries


def _validated_scores(raw_scores: Any) -> dict[str, float]:
    if not isinstance(raw_scores, Mapping) or not raw_scores:
        raise ValueError("row_report scores must be a non-empty object")

    scores: dict[str, float] = {}
    for raw_model_id, raw_score in sorted(raw_scores.items()):
        model_id = _required_model_id(raw_model_id, "model_id")
        scores[model_id] = _bounded_number(raw_score, f"{model_id} risk", 0.0, 1.0)
    return scores


def _validated_model_evidence(
    raw_evidence: Any,
    scores: Mapping[str, float],
) -> dict[str, list[Any]]:
    if raw_evidence is None:
        return {model_id: [] for model_id in sorted(scores)}
    if not isinstance(raw_evidence, Mapping):
        raise ValueError("evidence_by_model must be an object")

    evidence_by_model: dict[str, list[Any]] = {}
    for raw_model_id, raw_entries in sorted(raw_evidence.items()):
        model_id = _required_model_id(raw_model_id, "evidence model_id")
        if model_id not in scores:
            raise ValueError("evidence_by_model model_id must reference scores")
        evidence_by_model[model_id] = list(_bounded_list(raw_entries, "evidence_by_model"))

    for model_id in sorted(scores):
        evidence_by_model.setdefault(model_id, [])
    return evidence_by_model


def _supporting_models(row: Mapping[str, Any]) -> list[str]:
    return [
        model_id
        for model_id, score in sorted(row["scores"].items())
        if score >= SUPPORTING_MODEL_RISK_THRESHOLD
    ]


def _model_evidence_refs(
    row_index: int,
    row: Mapping[str, Any],
    model_ids: Sequence[str],
) -> list[JsonMap]:
    refs: list[JsonMap] = []
    for model_id in model_ids:
        for evidence_index, _entry in enumerate(row["evidence_by_model"].get(model_id, [])):
            refs.append(
                _evidence_ref(
                    report_schema=SOURCE_REPORT_SCHEMA_VERSION,
                    entity_id=row["entity_id"],
                    window_start=row["window_start"],
                    model_id=model_id,
                    row_index=row_index,
                    evidence_index=evidence_index,
                    field_path=(
                        f"[row_reports][{row_index}][evidence_by_model]"
                        f"[{model_id}][{evidence_index}]"
                    ),
                )
            )
    return refs


def _score_ref(row_index: int, row: Mapping[str, Any], model_id: str) -> JsonMap:
    return _evidence_ref(
        report_schema=SOURCE_REPORT_SCHEMA_VERSION,
        entity_id=row["entity_id"],
        window_start=row["window_start"],
        model_id=model_id,
        row_index=row_index,
        field_path=f"[row_reports][{row_index}][scores][{model_id}]",
    )


def _row_field_ref(row_index: int, row: Mapping[str, Any], model_id: str, field: str) -> JsonMap:
    return _evidence_ref(
        report_schema=SOURCE_REPORT_SCHEMA_VERSION,
        entity_id=row["entity_id"],
        window_start=row["window_start"],
        model_id=model_id,
        row_index=row_index,
        field_path=f"[row_reports][{row_index}][{field}]",
    )


def _evidence_ref(
    *,
    report_schema: str,
    entity_id: str,
    window_start: str,
    model_id: str,
    row_index: int,
    field_path: str,
    evidence_index: int | None = None,
) -> JsonMap:
    ref: JsonMap = {
        "report_schema": report_schema,
        "entity_id": entity_id,
        "window_start": window_start,
        "model_id": model_id,
        "row_index": row_index,
        "field_path": field_path,
    }
    if evidence_index is not None:
        ref["evidence_index"] = evidence_index
    _validate_evidence_ref(ref)
    return ref


def _limited_refs(refs: Sequence[Mapping[str, Any]]) -> list[JsonMap]:
    unique: list[JsonMap] = []
    seen: set[str] = set()
    for ref in refs:
        key = json.dumps(ref, sort_keys=True)
        if key in seen:
            continue
        seen.add(key)
        unique.append(dict(ref))
        if len(unique) >= MAX_REFS_PER_CANDIDATE:
            break
    return unique


def _validate_summary(summary: Any) -> None:
    if not isinstance(summary, Mapping):
        raise ValueError("validation_summary must be an object")
    _require_exact_fields(summary, VALIDATION_SUMMARY_FIELDS, "validation_summary")
    _non_negative_int(summary["source_rows_considered"], "source_rows_considered")
    _non_negative_int(summary["eligible_patterns"], "eligible_patterns")
    _non_negative_int(summary["candidates_generated"], "candidates_generated")
    languages = _bounded_list(summary["candidate_languages"], "candidate_languages")
    if tuple(languages) != CANDIDATE_LANGUAGES:
        raise ValueError("candidate_languages must list the v0 languages in stable order")
    _required_text(summary["validation_scope"], "validation_scope")
    if summary["human_review_required"] is not True:
        raise ValueError("validation_summary human_review_required must be true")
    if summary["deployment_allowed"] is not False:
        raise ValueError("validation_summary deployment_allowed must be false")


def _validate_candidate_row(row: Any) -> None:
    if not isinstance(row, Mapping):
        raise ValueError("candidate row must be an object")
    _require_exact_fields(row, CANDIDATE_ROW_FIELDS, "candidate row")
    candidate_id = _required_text(row["candidate_id"], "candidate_id")
    if not SAFE_CANDIDATE_ID_RE.fullmatch(candidate_id):
        raise ValueError("candidate_id must use stable cand-v0 format")
    if row["candidate_language"] not in CANDIDATE_LANGUAGES:
        raise ValueError("candidate_language is unsupported")
    _candidate_kind(row["candidate_kind"])
    _required_text(row["title"], "title")
    draft = _required_text(row["draft"], "draft")
    if DRAFT_MARKER not in draft:
        raise ValueError("draft must include DRAFT_DO_NOT_DEPLOY marker")

    refs = _bounded_list(row["source_evidence_refs"], "source_evidence_refs")
    if not refs:
        raise ValueError("source_evidence_refs must not be empty")
    for ref in refs:
        _validate_evidence_ref(ref)
    _validate_row_validation(row["validation"])
    _validate_false_positive(row["false_positive_estimate"])
    if row["human_review_required"] is not True:
        raise ValueError("human_review_required must be true")
    if row["deployment_allowed"] is not False:
        raise ValueError("deployment_allowed must be false")


def _validate_row_validation(validation: Any) -> None:
    if not isinstance(validation, Mapping):
        raise ValueError("validation must be an object")
    _require_exact_fields(validation, ROW_VALIDATION_FIELDS, "candidate validation")
    _non_negative_int(validation["source_row_index"], "source_row_index")
    _candidate_kind(validation["eligible_pattern"])
    if validation["candidate_language_checked"] not in CANDIDATE_LANGUAGES:
        raise ValueError("candidate_language_checked is unsupported")
    _required_text(validation["fixture_validation"], "fixture_validation")
    if validation["synthetic_source_required"] is not True:
        raise ValueError("synthetic_source_required must be true")
    if validation["validated_against_replay"] is not False:
        raise ValueError("validated_against_replay must be false in v0")


def _validate_false_positive(false_positive: Any) -> None:
    if not isinstance(false_positive, Mapping):
        raise ValueError("false_positive_estimate must be an object")
    _require_exact_fields(false_positive, FALSE_POSITIVE_FIELDS, "false_positive_estimate")
    if false_positive["label"] not in {"medium", "high"}:
        raise ValueError("false_positive_estimate label is unsupported")
    _required_text(false_positive["basis"], "false_positive_estimate basis")


def _validate_evidence_ref(ref: Any) -> None:
    if not isinstance(ref, Mapping):
        raise ValueError("evidence ref must be an object")
    actual_fields = set(ref)
    if not REF_REQUIRED_FIELDS <= actual_fields <= REF_ALLOWED_FIELDS:
        missing = sorted(REF_REQUIRED_FIELDS - actual_fields)
        unexpected = sorted(actual_fields - REF_ALLOWED_FIELDS)
        details = []
        if missing:
            details.append(f"missing {missing}")
        if unexpected:
            details.append(f"unexpected {unexpected}")
        raise ValueError(f"evidence ref fields invalid: {', '.join(details)}")

    if ref["report_schema"] != SOURCE_REPORT_SCHEMA_VERSION:
        raise ValueError("evidence ref report_schema is unsupported")
    _required_entity_id(ref["entity_id"], "entity_id")
    _required_window_start(ref["window_start"], "window_start")
    _required_model_id(ref["model_id"], "model_id")
    _non_negative_int(ref["row_index"], "row_index")
    field_path = _required_text(ref["field_path"], "field_path")
    if not SAFE_FIELD_PATH_RE.fullmatch(field_path):
        raise ValueError("field_path must use bracketed report field references")
    if "evidence_index" in ref:
        _non_negative_int(ref["evidence_index"], "evidence_index")


def _report_schema(report: Mapping[str, Any], expected: str) -> str:
    if not isinstance(report, Mapping):
        raise ValueError("report must be an object")
    schema = report.get("schema_version")
    if schema != expected:
        raise ValueError(f"unsupported report schema_version '{schema}'")
    return str(schema)


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


def _required_entity_id(value: Any, field: str) -> str:
    text = _required_text(value, field)
    if not SAFE_ENTITY_ID_RE.fullmatch(text):
        raise ValueError(
            f"{field} must be a synthetic/coarse entity identifier, "
            "not a raw hostname, address, username, or private identifier"
        )
    return text


def _required_model_id(value: Any, field: str) -> str:
    text = _required_text(value, field)
    if not SAFE_MODEL_ID_RE.fullmatch(text):
        raise ValueError(f"{field} must be a sanitized model identifier")
    return text


def _candidate_kind(value: Any) -> str:
    text = _required_text(value, "candidate_kind")
    if text not in CANDIDATE_KINDS:
        raise ValueError("candidate_kind is unsupported")
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


def _bounded_number(raw_value: Any, field: str, lower: float, upper: float) -> float:
    value = _finite_number(raw_value, field)
    if value < lower or value > upper:
        raise ValueError(f"{field} must be between {lower} and {upper}")
    return value


def _finite_number(raw_value: Any, field: str) -> float:
    if isinstance(raw_value, bool) or not isinstance(raw_value, int | float):
        raise ValueError(f"{field} must be a finite number")
    value = float(raw_value)
    if not math.isfinite(value):
        raise ValueError(f"{field} must be a finite number")
    return value


def _non_negative_int(raw_value: Any, field: str) -> int:
    if isinstance(raw_value, bool) or not isinstance(raw_value, int):
        raise ValueError(f"{field} must be a non-negative integer")
    if raw_value < 0:
        raise ValueError(f"{field} must be a non-negative integer")
    return raw_value


def _model_phrase(model_ids: Sequence[str]) -> str:
    shown = list(model_ids[:4])
    suffix = f"_plus_{len(model_ids) - len(shown)}" if len(model_ids) > len(shown) else ""
    return ",".join(shown) + suffix


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Generate deterministic draft detection engineering candidates."
    )
    parser.add_argument("disagreement_report", help="model_disagreement_report.v0 JSON")
    parser.add_argument("output", help="Path to write detection candidate report JSON")
    args = parser.parse_args(argv)

    report = generate_candidate_report(load_report(args.disagreement_report))
    dump_report(report, args.output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
