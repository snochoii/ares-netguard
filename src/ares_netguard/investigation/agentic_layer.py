"""Deterministic offline agentic investigation report generation.

The v0 layer works only over local, already-generated synthetic reports. It
does not call LLMs, use network access, perform probing, deploy rules, or make
final incident decisions.
"""

from __future__ import annotations

import argparse
import json
import math
import re
from collections.abc import Mapping, Sequence
from datetime import datetime
from pathlib import Path
from typing import Any

REPORT_SCHEMA_VERSION = "agentic_investigation_report.v0"
PRIMARY_REPORT_SCHEMA_VERSION = "model_disagreement_report.v0"
SUPPORTED_EVIDENCE_REPORT_SCHEMAS = frozenset(
    {
        "time_series_residual_report.v0",
        "traffic_representation_report.v0",
        "temporal_security_graph_report.v0",
    }
)
SUPPORTED_REPORT_SCHEMAS = {
    REPORT_SCHEMA_VERSION,
    PRIMARY_REPORT_SCHEMA_VERSION,
} | SUPPORTED_EVIDENCE_REPORT_SCHEMAS

HIGH_CONSENSUS_RISK_THRESHOLD = 0.75
SUPPORTING_MODEL_RISK_THRESHOLD = 0.7
HIGH_DISAGREEMENT_THRESHOLD = 0.5
SPARSE_EVIDENCE_MIN_MODELS = 2

MAX_STRING_LENGTH = 256
MAX_LIST_LENGTH = 1000
MAX_MAPPING_LENGTH = 128
MAX_DEPTH = 16
MAX_REFS_PER_HYPOTHESIS = 12

REPORT_FIELDS = frozenset(
    {
        "schema_version",
        "primary_report_schema",
        "evidence_report_schemas",
        "rows",
    }
)
HYPOTHESIS_ROW_FIELDS = frozenset(
    {
        "hypothesis_id",
        "claim",
        "supporting_evidence_refs",
        "refuting_evidence_refs",
        "missing_evidence",
        "confidence",
        "recommended_next_query",
        "human_review_required",
    }
)
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
SAFE_HYPOTHESIS_ID_RE = re.compile(r"^hyp-[0-9]{4}$")
SAFE_FIELD_PATH_RE = re.compile(r"^\[[A-Za-z0-9_-]+](?:\[[A-Za-z0-9_-]+])*$")
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

RISK_FIELD_BY_SCHEMA = {
    "time_series_residual_report.v0": "residual_risk",
    "traffic_representation_report.v0": "representation_risk",
    "temporal_security_graph_report.v0": "graph_novelty_risk",
}


def load_report(path: str | Path) -> JsonMap:
    """Load a supported report JSON file using strict JSON constants."""
    source = Path(path)
    if source.is_dir():
        raise ValueError(f"report path must be a file, not a directory: {source}")

    payload = _loads_strict(source.read_text(encoding="utf-8"))
    if not isinstance(payload, Mapping):
        raise ValueError(f"report payload must be an object: {source}")

    report = dict(payload)
    schema_version = report.get("schema_version")
    if schema_version not in SUPPORTED_REPORT_SCHEMAS:
        raise ValueError(f"unknown report schema_version '{schema_version}'")
    _validate_safe_tree(report, "report")
    return report


def generate_investigation_report(
    disagreement_report: Mapping[str, Any],
    *,
    evidence_reports: Sequence[Mapping[str, Any]] = (),
) -> JsonMap:
    """Generate deterministic investigation hypotheses from local reports."""
    row_entries = _validated_disagreement_rows(disagreement_report)
    local_evidence_by_window = _local_evidence_refs_by_window(evidence_reports)

    hypotheses: list[JsonMap] = []
    for row_index, row in row_entries:
        local_refs = local_evidence_by_window.get((row["entity_id"], row["window_start"]), [])
        hypotheses.extend(_high_consensus_hypotheses(row_index, row, local_refs))
        hypotheses.extend(_high_disagreement_hypotheses(row_index, row))
        hypotheses.extend(_sparse_evidence_hypotheses(row_index, row))
        hypotheses.extend(_local_evidence_match_hypotheses(row_index, row, local_refs))

    rows = []
    for hypothesis_number, hypothesis in enumerate(hypotheses, start=1):
        hypothesis["hypothesis_id"] = f"hyp-{hypothesis_number:04d}"
        rows.append(hypothesis)

    evidence_schemas = sorted(
        {_report_schema(report, SUPPORTED_EVIDENCE_REPORT_SCHEMAS) for report in evidence_reports}
    )
    report = {
        "schema_version": REPORT_SCHEMA_VERSION,
        "primary_report_schema": PRIMARY_REPORT_SCHEMA_VERSION,
        "evidence_report_schemas": evidence_schemas,
        "rows": rows,
    }
    validate_investigation_report(report)
    return report


def validate_investigation_report(report: Mapping[str, Any]) -> None:
    """Validate the strict v0 agentic investigation report contract."""
    _validate_safe_tree(report, "report")
    _require_exact_fields(report, REPORT_FIELDS, "investigation report")
    if report.get("schema_version") != REPORT_SCHEMA_VERSION:
        raise ValueError(f"investigation report requires schema_version '{REPORT_SCHEMA_VERSION}'")
    if report.get("primary_report_schema") != PRIMARY_REPORT_SCHEMA_VERSION:
        raise ValueError(f"primary_report_schema must be '{PRIMARY_REPORT_SCHEMA_VERSION}'")

    evidence_schemas = _bounded_list(
        report.get("evidence_report_schemas"), "evidence_report_schemas"
    )
    for schema in evidence_schemas:
        if schema not in SUPPORTED_EVIDENCE_REPORT_SCHEMAS:
            raise ValueError(f"unsupported evidence_report_schema '{schema}'")

    rows = _bounded_list(report.get("rows"), "rows")
    for row in rows:
        _validate_hypothesis_row(row)


def dump_report(report: Mapping[str, Any], path: str | Path) -> None:
    """Write report JSON with stable formatting and strict finite numbers."""
    validate_investigation_report(report)
    Path(path).write_text(
        json.dumps(report, allow_nan=False, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def _high_consensus_hypotheses(
    row_index: int,
    row: Mapping[str, Any],
    local_refs: Sequence[Mapping[str, Any]],
) -> list[JsonMap]:
    supporting_models = _supporting_models(row)
    if row["consensus_risk"] < HIGH_CONSENSUS_RISK_THRESHOLD or len(supporting_models) < 2:
        return []

    support_refs = [_score_ref(row_index, row, model_id) for model_id in supporting_models]
    support_refs.extend(_model_evidence_refs(row_index, row, supporting_models))
    support_refs.extend(local_refs)

    missing_evidence = ["human analyst validation of the hypothesis"]
    if not local_refs:
        missing_evidence.append("matching local evidence report for entity/window")

    return [
        _hypothesis(
            claim=(
                f"Entity {row['entity_id']} shows high consensus anomaly risk "
                f"across {len(supporting_models)} models for window {row['window_start']}."
            ),
            supporting_evidence_refs=_limited_refs(support_refs),
            refuting_evidence_refs=[],
            missing_evidence=missing_evidence,
            confidence=min(0.95, row["consensus_risk"]),
            recommended_next_query=(
                f"Review local evidence refs for {row['entity_id']} at "
                f"{row['window_start']} and compare supporting model reasons."
            ),
        )
    ]


def _high_disagreement_hypotheses(row_index: int, row: Mapping[str, Any]) -> list[JsonMap]:
    outlier_model = row.get("outlier_model")
    if (
        row["disagreement_score"] < HIGH_DISAGREEMENT_THRESHOLD
        or not isinstance(outlier_model, str)
        or not outlier_model
    ):
        return []

    refuting_models = [model_id for model_id in sorted(row["scores"]) if model_id != outlier_model]
    return [
        _hypothesis(
            claim=(
                f"Entity {row['entity_id']} has high model disagreement for window "
                f"{row['window_start']}; {outlier_model} is the outlier model."
            ),
            supporting_evidence_refs=_limited_refs(
                [
                    _score_ref(row_index, row, outlier_model),
                    _row_field_ref(row_index, row, outlier_model, "disagreement_score"),
                    *_model_evidence_refs(row_index, row, [outlier_model]),
                ]
            ),
            refuting_evidence_refs=_limited_refs(
                [_score_ref(row_index, row, model_id) for model_id in refuting_models]
            ),
            missing_evidence=[
                f"explanation for {outlier_model} divergence",
                "human analyst validation of the disagreement",
            ],
            confidence=min(0.9, row["disagreement_score"]),
            recommended_next_query=(
                f"Compare {outlier_model} evidence against peer model evidence for "
                f"{row['entity_id']} at {row['window_start']}."
            ),
        )
    ]


def _sparse_evidence_hypotheses(row_index: int, row: Mapping[str, Any]) -> list[JsonMap]:
    evidence_by_model = row["evidence_by_model"]
    evidence_model_count = sum(1 for values in evidence_by_model.values() if values)
    supporting_models = _supporting_models(row)
    if evidence_model_count >= SPARSE_EVIDENCE_MIN_MODELS and len(supporting_models) >= 2:
        return []

    missing_model_evidence = [
        model_id for model_id in sorted(row["scores"]) if not evidence_by_model.get(model_id)
    ]
    missing_evidence = [
        "additional independent model or telemetry evidence",
        "matching local evidence report for entity/window",
        "human analyst validation before incident handling",
    ]
    if missing_model_evidence:
        missing_evidence.append(f"model evidence for {', '.join(missing_model_evidence)}")

    return [
        _hypothesis(
            claim=(
                f"Entity {row['entity_id']} has sparse or missing supporting evidence "
                f"for window {row['window_start']}."
            ),
            supporting_evidence_refs=_limited_refs(
                [
                    _row_field_ref(row_index, row, "model_disagreement", "consensus_risk"),
                    *[_score_ref(row_index, row, model_id) for model_id in sorted(row["scores"])],
                ]
            ),
            refuting_evidence_refs=[],
            missing_evidence=missing_evidence,
            confidence=0.35,
            recommended_next_query=(
                f"Gather additional local evidence reports for {row['entity_id']} at "
                f"{row['window_start']} before escalating."
            ),
        )
    ]


def _local_evidence_match_hypotheses(
    row_index: int,
    row: Mapping[str, Any],
    local_refs: Sequence[Mapping[str, Any]],
) -> list[JsonMap]:
    if not local_refs:
        return []

    return [
        _hypothesis(
            claim=(
                f"Entity {row['entity_id']} has matching local evidence reports for "
                f"window {row['window_start']}."
            ),
            supporting_evidence_refs=_limited_refs(
                [
                    _row_field_ref(row_index, row, "model_disagreement", "consensus_risk"),
                    *local_refs,
                ]
            ),
            refuting_evidence_refs=[],
            missing_evidence=["human analyst validation of local evidence relevance"],
            confidence=0.55,
            recommended_next_query=(
                f"Inspect referenced local evidence rows for {row['entity_id']} at "
                f"{row['window_start']} and verify they explain the model signal."
            ),
        )
    ]


def _hypothesis(
    *,
    claim: str,
    supporting_evidence_refs: Sequence[Mapping[str, Any]],
    refuting_evidence_refs: Sequence[Mapping[str, Any]],
    missing_evidence: Sequence[str],
    confidence: float,
    recommended_next_query: str,
) -> JsonMap:
    return {
        "hypothesis_id": "",
        "claim": claim,
        "supporting_evidence_refs": [dict(ref) for ref in supporting_evidence_refs],
        "refuting_evidence_refs": [dict(ref) for ref in refuting_evidence_refs],
        "missing_evidence": list(missing_evidence),
        "confidence": _round(confidence),
        "recommended_next_query": recommended_next_query,
        "human_review_required": True,
    }


def _validated_disagreement_rows(report: Mapping[str, Any]) -> list[tuple[int, JsonMap]]:
    _report_schema(report, {PRIMARY_REPORT_SCHEMA_VERSION})
    _validate_safe_tree(report, "model disagreement report")

    rows = _bounded_list(report.get("row_reports"), "row_reports")
    row_entries: list[tuple[int, JsonMap]] = []
    for row_index, raw_row in enumerate(rows):
        if not isinstance(raw_row, Mapping):
            raise ValueError("row_reports entries must be objects")

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
        entries = _bounded_list(raw_entries, f"evidence_by_model[{model_id}]")
        evidence_by_model[model_id] = list(entries)

    for model_id in sorted(scores):
        evidence_by_model.setdefault(model_id, [])
    return evidence_by_model


def _local_evidence_refs_by_window(
    evidence_reports: Sequence[Mapping[str, Any]],
) -> dict[tuple[str, str], list[JsonMap]]:
    refs_by_window: dict[tuple[str, str], list[JsonMap]] = {}
    for report in evidence_reports:
        schema = _report_schema(report, SUPPORTED_EVIDENCE_REPORT_SCHEMAS)
        _validate_safe_tree(report, f"{schema} report")
        rows = _bounded_list(report.get("rows"), f"{schema} rows")
        report_model_id = _required_model_id(report.get("model_id"), "model_id")
        risk_field = RISK_FIELD_BY_SCHEMA[schema]

        for row_index, row in enumerate(rows):
            if not isinstance(row, Mapping):
                raise ValueError(f"{schema} rows entries must be objects")
            entity_id = _required_entity_id(row.get("entity_id"), "entity_id")
            window_start = _required_window_start(row.get("window_start"), "window_start")
            row_model_id = _required_model_id(row.get("model_id", report_model_id), "model_id")
            if row_model_id != report_model_id:
                raise ValueError(f"{schema} row model_id must match report model_id")
            _bounded_number(row.get(risk_field), risk_field, 0.0, 1.0)

            ref = _evidence_ref(
                report_schema=schema,
                entity_id=entity_id,
                window_start=window_start,
                model_id=row_model_id,
                row_index=row_index,
                field_path=f"[rows][{row_index}][{risk_field}]",
            )
            refs_by_window.setdefault((entity_id, window_start), []).append(ref)

    for refs in refs_by_window.values():
        refs.sort(
            key=lambda ref: (
                ref["report_schema"],
                ref["model_id"],
                ref["row_index"],
                ref["field_path"],
            )
        )
    return refs_by_window


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
                    report_schema=PRIMARY_REPORT_SCHEMA_VERSION,
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
        report_schema=PRIMARY_REPORT_SCHEMA_VERSION,
        entity_id=row["entity_id"],
        window_start=row["window_start"],
        model_id=model_id,
        row_index=row_index,
        field_path=f"[row_reports][{row_index}][scores][{model_id}]",
    )


def _row_field_ref(row_index: int, row: Mapping[str, Any], model_id: str, field: str) -> JsonMap:
    return _evidence_ref(
        report_schema=PRIMARY_REPORT_SCHEMA_VERSION,
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
        if len(unique) >= MAX_REFS_PER_HYPOTHESIS:
            break
    return unique


def _validate_hypothesis_row(row: Any) -> None:
    if not isinstance(row, Mapping):
        raise ValueError("hypothesis row must be an object")
    _require_exact_fields(row, HYPOTHESIS_ROW_FIELDS, "hypothesis row")
    if not isinstance(row["hypothesis_id"], str) or not SAFE_HYPOTHESIS_ID_RE.fullmatch(
        row["hypothesis_id"]
    ):
        raise ValueError("hypothesis_id must use 'hyp-0000' format")
    _required_text(row["claim"], "claim")
    _required_text(row["recommended_next_query"], "recommended_next_query")
    _bounded_number(row["confidence"], "confidence", 0.0, 1.0)
    if row["human_review_required"] is not True:
        raise ValueError("human_review_required must be true")

    missing_evidence = _bounded_list(row["missing_evidence"], "missing_evidence")
    if not missing_evidence:
        raise ValueError("missing_evidence must not be empty")
    for item in missing_evidence:
        _required_text(item, "missing_evidence")

    supporting_refs = _bounded_list(row["supporting_evidence_refs"], "supporting_evidence_refs")
    if not supporting_refs:
        raise ValueError("supporting_evidence_refs must not be empty")
    for ref in supporting_refs:
        _validate_evidence_ref(ref)

    refuting_refs = _bounded_list(row["refuting_evidence_refs"], "refuting_evidence_refs")
    for ref in refuting_refs:
        _validate_evidence_ref(ref)


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

    if ref["report_schema"] not in SUPPORTED_REPORT_SCHEMAS:
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


def _report_schema(report: Mapping[str, Any], allowed: set[str] | frozenset[str]) -> str:
    if not isinstance(report, Mapping):
        raise ValueError("report must be an object")
    schema = report.get("schema_version")
    if schema not in allowed:
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
    ):
        raise ValueError(f"{field} contains unsafe raw identifier content")


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


def _round(value: float) -> float:
    rounded = round(value, 6)
    return 0.0 if rounded == 0 else rounded


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Generate deterministic offline agentic investigation hypotheses."
    )
    parser.add_argument("disagreement_report", help="model_disagreement_report.v0 JSON")
    parser.add_argument("output", help="Path to write agentic investigation report JSON")
    parser.add_argument(
        "--evidence-report",
        action="append",
        default=[],
        help="Optional local evidence report JSON; may be repeated",
    )
    args = parser.parse_args(argv)

    report = generate_investigation_report(
        load_report(args.disagreement_report),
        evidence_reports=[load_report(path) for path in args.evidence_report],
    )
    dump_report(report, args.output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
