"""Compose strict model_score_row.v0 sources for disagreement smoke flows.

The v0 composer only merges already-generated synthetic score rows and report
contracts. It does not train models, execute native runtimes, inspect packet
payloads, perform capture, or call external services.
"""

from __future__ import annotations

import argparse
import json
import math
from collections.abc import Mapping, Sequence
from pathlib import Path
from typing import Any

from ares_netguard.graph import temporal_security_graph
from ares_netguard.models import (
    evaluation_bundle,
    self_supervised_representation,
    time_series_residual,
)
from ares_netguard.models.disagreement import (
    ROW_SCHEMA_VERSION,
    generate_disagreement_report,
)

JsonMap = dict[str, Any]

SCORE_ROW_FIELDS = frozenset({"schema_version", "entity_id", "window_start", "scores"})


def load_score_rows(path: str | Path) -> list[JsonMap]:
    """Load JSON or JSONL model_score_row.v0 rows with strict JSON constants."""
    source = _input_file(path)
    text = source.read_text(encoding="utf-8").strip()
    if not text:
        return []

    if source.suffix == ".jsonl":
        rows = [_loads_strict(line) for line in text.splitlines() if line.strip()]
    else:
        payload = _loads_strict(text)
        rows = _score_rows_from_payload(payload, source)

    loaded: list[JsonMap] = []
    for index, row in enumerate(rows):
        validate_score_row(row, label=f"score row {index}")
        loaded.append(_clone_json(row))
    return loaded


def load_residual_report(path: str | Path) -> JsonMap:
    """Load and validate a strict time_series_residual_report.v0/v1 source."""
    report = _load_report_object(path, "residual report")
    time_series_residual.validate_residual_report(report)
    time_series_residual.residual_evidence_to_score_rows(report)
    return report


def load_representation_report(path: str | Path) -> JsonMap:
    """Load and validate a traffic_representation_report.v0 source."""
    report = _load_report_object(path, "representation report")
    self_supervised_representation.representation_evidence_to_score_rows(report)
    return report


def load_graph_report(path: str | Path) -> JsonMap:
    """Load and validate a temporal_security_graph_report.v0 source."""
    report = _load_report_object(path, "graph report")
    temporal_security_graph.temporal_graph_evidence_to_score_rows(report)
    return report


def compose_score_rows(
    score_rows: Sequence[Mapping[str, Any]] | None = None,
    *,
    score_row_sources: Sequence[Sequence[Mapping[str, Any]]] = (),
    residual_reports: Sequence[Mapping[str, Any]] = (),
    representation_reports: Sequence[Mapping[str, Any]] = (),
    graph_reports: Sequence[Mapping[str, Any]] = (),
) -> list[JsonMap]:
    """Merge score-row sources by entity/window while preserving model evidence.

    A duplicate ``(entity_id, window_start, model_id)`` tuple is rejected even if
    the duplicate payload is identical. That fail-closed behavior keeps the
    primary disagreement smoke input unambiguous.
    """
    grouped: dict[tuple[str, str], JsonMap] = {}
    sources: list[Sequence[Mapping[str, Any]]] = []
    if score_rows is not None:
        sources.append(score_rows)
    sources.extend(score_row_sources)

    for source in sources:
        _merge_rows(grouped, source)
    for report in residual_reports:
        _merge_rows(grouped, time_series_residual.residual_evidence_to_score_rows(report))
    for report in representation_reports:
        _merge_rows(
            grouped,
            self_supervised_representation.representation_evidence_to_score_rows(report),
        )
    for report in graph_reports:
        _merge_rows(grouped, temporal_security_graph.temporal_graph_evidence_to_score_rows(report))

    composed: list[JsonMap] = []
    for key in sorted(grouped):
        row = grouped[key]
        row["scores"] = {model_id: row["scores"][model_id] for model_id in sorted(row["scores"])}
        validate_score_row(row)
        composed.append(row)
    return composed


def validate_score_row(row: Mapping[str, Any], *, label: str = "score row") -> None:
    """Validate a strict model_score_row.v0 row accepted by the composer."""
    if not isinstance(row, Mapping):
        raise ValueError(f"{label} must be an object")
    _require_exact_fields(row, SCORE_ROW_FIELDS, label)
    _validate_json_tree(row, label)
    evaluation_bundle._validate_safe_tree(row, label)

    if row["schema_version"] != ROW_SCHEMA_VERSION:
        raise ValueError(f"{label} requires schema_version '{ROW_SCHEMA_VERSION}'")
    _required_text(row["entity_id"], f"{label}.entity_id")
    _required_text(row["window_start"], f"{label}.window_start")

    scores = row["scores"]
    if not isinstance(scores, Mapping) or not scores:
        raise ValueError(f"{label} requires a non-empty scores object")
    for model_id in scores:
        _required_text(model_id, f"{label}.scores model_id")

    evaluation_bundle._validate_score_rows([row])
    # Reuse the disagreement engine's score-scale validation so the composer
    # accepts exactly the rows that can feed the primary report.
    generate_disagreement_report([row])


def dump_score_rows(rows: Sequence[Mapping[str, Any]], path: str | Path) -> None:
    """Write a bare strict JSON list of model_score_row.v0 rows."""
    output = Path(path)
    if output.is_dir():
        raise ValueError(f"output path must be a file, not a directory: {output}")
    rows_to_write = [_clone_json(row) for row in rows]
    for index, row in enumerate(rows_to_write):
        validate_score_row(row, label=f"score row {index}")
    output.write_text(
        json.dumps(rows_to_write, allow_nan=False, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def _merge_rows(
    grouped: dict[tuple[str, str], JsonMap],
    rows: Sequence[Mapping[str, Any]],
) -> None:
    for row in rows:
        validate_score_row(row)
        entity_id = str(row["entity_id"])
        window_start = str(row["window_start"])
        key = (entity_id, window_start)
        target = grouped.setdefault(
            key,
            {
                "schema_version": ROW_SCHEMA_VERSION,
                "entity_id": entity_id,
                "window_start": window_start,
                "scores": {},
            },
        )
        target_scores = target["scores"]
        if not isinstance(target_scores, dict):
            raise ValueError("internal score merge state is invalid")
        for model_id, score_entry in row["scores"].items():
            if model_id in target_scores:
                raise ValueError(
                    "duplicate score tuple for "
                    f"entity_id={entity_id!r}, window_start={window_start!r}, "
                    f"model_id={model_id!r}"
                )
            target_scores[str(model_id)] = _clone_json(score_entry)


def _score_rows_from_payload(payload: Any, source: Path) -> list[Any]:
    if isinstance(payload, list):
        return payload
    if isinstance(payload, Mapping) and isinstance(payload.get("rows"), list):
        return list(payload["rows"])
    if isinstance(payload, Mapping):
        return [payload]
    raise ValueError(f"unsupported score row payload in {source}")


def _load_report_object(path: str | Path, label: str) -> JsonMap:
    source = _input_file(path)
    payload = _loads_strict(source.read_text(encoding="utf-8"))
    if not isinstance(payload, Mapping):
        raise ValueError(f"{label} payload must be an object: {source}")
    _validate_json_tree(payload, label)
    evaluation_bundle._validate_safe_tree(payload, label)
    return dict(payload)


def _input_file(path: str | Path) -> Path:
    source = Path(path)
    if source.is_dir():
        raise ValueError(f"input path must be a file, not a directory: {source}")
    return source


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


def _required_text(value: Any, field: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise ValueError(f"{field} must be a non-empty string")
    return value


def _validate_json_tree(value: Any, label: str, *, depth: int = 0) -> None:
    if depth > 32:
        raise ValueError(f"{label} exceeds maximum nesting depth")
    if isinstance(value, Mapping):
        for key, item in value.items():
            if not isinstance(key, str) or not key:
                raise ValueError(f"{label} object keys must be non-empty strings")
            _validate_json_tree(item, f"{label}.{key}", depth=depth + 1)
        return
    if isinstance(value, list):
        for index, item in enumerate(value):
            _validate_json_tree(item, f"{label}[{index}]", depth=depth + 1)
        return
    if isinstance(value, str) or value is None or isinstance(value, bool):
        return
    if isinstance(value, int | float):
        if not math.isfinite(float(value)):
            raise ValueError(f"{label} must contain only finite numbers")
        return
    raise ValueError(f"{label} contains unsupported JSON value type")


def _clone_json(value: Any) -> Any:
    _validate_json_tree(value, "JSON value")
    return json.loads(json.dumps(value, allow_nan=False, sort_keys=True))


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Compose model_score_row.v0 rows from score-row and report sources."
    )
    parser.add_argument("output", help="Path to write a bare model_score_row.v0 JSON list")
    parser.add_argument(
        "--score-rows",
        action="append",
        default=[],
        help="JSON or JSONL model_score_row.v0 source; may be repeated",
    )
    parser.add_argument(
        "--residual-report",
        action="append",
        default=[],
        help="time_series_residual_report.v0/v1 JSON source; may be repeated",
    )
    parser.add_argument(
        "--representation-report",
        action="append",
        default=[],
        help="traffic_representation_report.v0 JSON source; may be repeated",
    )
    parser.add_argument(
        "--graph-report",
        action="append",
        default=[],
        help="temporal_security_graph_report.v0 JSON source; may be repeated",
    )
    args = parser.parse_args(argv)

    rows = compose_score_rows(
        score_row_sources=[load_score_rows(path) for path in args.score_rows],
        residual_reports=[load_residual_report(path) for path in args.residual_report],
        representation_reports=[
            load_representation_report(path) for path in args.representation_report
        ],
        graph_reports=[load_graph_report(path) for path in args.graph_report],
    )
    dump_score_rows(rows, args.output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
