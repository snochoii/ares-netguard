"""Deterministic model disagreement report generation.

The v0 contract consumes already-produced model score rows. It does not train
models, inspect packet payloads, perform capture, or call external services.
"""

from __future__ import annotations

import argparse
import json
import math
from collections import defaultdict
from collections.abc import Iterable, Mapping, Sequence
from pathlib import Path
from typing import Any

REPORT_SCHEMA_VERSION = "model_disagreement_report.v0"
ROW_SCHEMA_VERSION = "model_score_row.v0"
OUTLIER_DEVIATION_THRESHOLD = 0.25

JsonMap = dict[str, Any]


def load_score_rows(path: str | Path) -> list[JsonMap]:
    """Load JSON or JSONL model score rows."""
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
    raise ValueError(f"unsupported score row payload in {source}")


def generate_disagreement_report(rows: Sequence[Mapping[str, Any]]) -> JsonMap:
    """Generate a deterministic model disagreement report from score rows."""
    row_reports = [_summarize_row(row) for row in rows]
    model_matrix = [
        {
            "entity_id": row_report["entity_id"],
            "window_start": row_report["window_start"],
            "scores": row_report["scores"],
        }
        for row_report in row_reports
    ]

    evidence_by_model = _collect_evidence(row_reports)
    model_stats = _summarize_models(row_reports)
    outlier_model = (
        model_stats[0]["model_id"]
        if model_stats and model_stats[0]["mean_deviation"] >= OUTLIER_DEVIATION_THRESHOLD
        else None
    )
    outlier_models = [
        entry["model_id"]
        for entry in model_stats
        if entry["mean_deviation"] >= OUTLIER_DEVIATION_THRESHOLD
    ]

    return {
        "schema_version": REPORT_SCHEMA_VERSION,
        "model_score_matrix": model_matrix,
        "model_agreement_score": _mean(row["agreement_score"] for row in row_reports),
        "model_disagreement_score": _mean(row["disagreement_score"] for row in row_reports),
        "consensus_risk": _mean(row["consensus_risk"] for row in row_reports),
        "outlier_model": outlier_model,
        "outlier_models": outlier_models,
        "top_supporting_models": _top_supporting_models(row_reports),
        "top_dissenting_models": model_stats,
        "evidence_by_model": evidence_by_model,
        "row_reports": row_reports,
    }


def dump_report(report: Mapping[str, Any], path: str | Path) -> None:
    """Write report JSON with stable formatting."""
    Path(path).write_text(
        json.dumps(report, allow_nan=False, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def _summarize_row(row: Mapping[str, Any]) -> JsonMap:
    _validate_row(row)
    scores = _normalize_scores(row)
    risks = list(scores.values())
    consensus = _mean(risks)
    disagreement = max(risks) - min(risks) if len(risks) > 1 else 0.0
    deviations = {model_id: abs(score - consensus) for model_id, score in scores.items()}

    max_deviation = max(deviations.values(), default=0.0)
    outliers = [
        model_id
        for model_id, deviation in sorted(deviations.items())
        if max_deviation >= OUTLIER_DEVIATION_THRESHOLD and deviation == max_deviation
    ]

    return {
        "schema_version": ROW_SCHEMA_VERSION,
        "entity_id": _required_text(row, "entity_id"),
        "window_start": _required_text(row, "window_start"),
        "scores": scores,
        "agreement_score": _round(1.0 - disagreement),
        "disagreement_score": _round(disagreement),
        "consensus_risk": _round(consensus),
        "outlier_model": outliers[0] if outliers else None,
        "outlier_models": outliers,
        "evidence_by_model": _row_evidence(row),
    }


def _normalize_scores(row: Mapping[str, Any]) -> dict[str, float]:
    raw_scores = row.get("scores")
    if not isinstance(raw_scores, Mapping) or not raw_scores:
        raise ValueError("score row requires a non-empty 'scores' object")

    normalized: dict[str, float] = {}
    for model_id, raw_entry in sorted(raw_scores.items()):
        normalized[model_id] = _normalize_score_entry(model_id, raw_entry)
    return normalized


def _normalize_score_entry(model_id: str, raw_entry: Any) -> float:
    if isinstance(raw_entry, int | float):
        return _risk_value(model_id, _numeric_score(model_id, raw_entry, "risk"))

    if not isinstance(raw_entry, Mapping):
        raise ValueError(f"{model_id}: score entry must be a number or object")

    scale = str(raw_entry.get("scale", "risk"))
    raw_value = raw_entry.get("risk", raw_entry.get("score"))
    if raw_value is None:
        raise ValueError(f"{model_id}: score entry requires 'risk' or 'score'")

    value = _numeric_score(model_id, raw_value, "score")
    match scale:
        case "risk" | "anomaly" | "confidence":
            risk = value
        case "inverted_risk":
            risk = 1.0 - _bounded_value(model_id, value, 0.0, 1.0, scale)
        case "percentile":
            risk = _bounded_value(model_id, value, 0.0, 100.0, scale) / 100.0
        case _:
            raise ValueError(f"{model_id}: unsupported score scale '{scale}'")
    return _risk_value(model_id, risk)


def _risk_value(model_id: str, value: float) -> float:
    return _round(_bounded_value(model_id, value, 0.0, 1.0, "risk"))


def _bounded_value(model_id: str, value: float, lower: float, upper: float, scale: str) -> float:
    if value < lower or value > upper:
        raise ValueError(f"{model_id}: {scale} score {value} outside {lower}..{upper}")
    return value


def _numeric_score(model_id: str, raw_value: Any, field: str) -> float:
    if isinstance(raw_value, bool) or not isinstance(raw_value, int | float):
        raise ValueError(f"{model_id}: {field} must be a finite number")

    value = float(raw_value)
    if not math.isfinite(value):
        raise ValueError(f"{model_id}: {field} must be a finite number")
    return value


def _row_evidence(row: Mapping[str, Any]) -> dict[str, list[Any]]:
    raw_scores = row.get("scores")
    if not isinstance(raw_scores, Mapping):
        return {}

    evidence: dict[str, list[Any]] = {}
    for model_id, raw_entry in sorted(raw_scores.items()):
        if isinstance(raw_entry, Mapping):
            raw_evidence = raw_entry.get("evidence", [])
            if raw_evidence is None:
                evidence[model_id] = []
            elif isinstance(raw_evidence, list):
                evidence[model_id] = raw_evidence
            else:
                evidence[model_id] = [raw_evidence]
        else:
            evidence[model_id] = []
    return evidence


def _collect_evidence(row_reports: Sequence[Mapping[str, Any]]) -> dict[str, list[Any]]:
    evidence: dict[str, list[Any]] = defaultdict(list)
    seen: dict[str, set[str]] = defaultdict(set)

    for row_report in row_reports:
        for model_id, entries in row_report["evidence_by_model"].items():
            for entry in entries:
                key = json.dumps(entry, sort_keys=True)
                if key not in seen[model_id]:
                    seen[model_id].add(key)
                    evidence[model_id].append(entry)

    return {model_id: evidence[model_id] for model_id in sorted(evidence)}


def _summarize_models(row_reports: Sequence[Mapping[str, Any]]) -> list[JsonMap]:
    deviations: dict[str, list[float]] = defaultdict(list)
    for row_report in row_reports:
        consensus = row_report["consensus_risk"]
        for model_id, score in row_report["scores"].items():
            deviations[model_id].append(abs(score - consensus))

    stats = [
        {
            "model_id": model_id,
            "mean_deviation": _round(_mean(values)),
        }
        for model_id, values in deviations.items()
    ]
    return sorted(stats, key=lambda item: (-item["mean_deviation"], item["model_id"]))


def _top_supporting_models(row_reports: Sequence[Mapping[str, Any]]) -> list[JsonMap]:
    risks: dict[str, list[float]] = defaultdict(list)
    for row_report in row_reports:
        for model_id, score in row_report["scores"].items():
            risks[model_id].append(score)

    stats = [
        {
            "model_id": model_id,
            "mean_risk": _round(_mean(values)),
        }
        for model_id, values in risks.items()
    ]
    return sorted(stats, key=lambda item: (-item["mean_risk"], item["model_id"]))


def _required_text(row: Mapping[str, Any], key: str) -> str:
    value = row.get(key)
    if not isinstance(value, str) or not value.strip():
        raise ValueError(f"score row requires non-empty '{key}'")
    return value


def _validate_row(row: Mapping[str, Any]) -> None:
    if not isinstance(row, Mapping):
        raise ValueError("score row must be an object")
    if row.get("schema_version") != ROW_SCHEMA_VERSION:
        raise ValueError(f"score row requires schema_version '{ROW_SCHEMA_VERSION}'")


def _mean(values: Iterable[float]) -> float:
    items = list(values)
    if not items:
        return 0.0
    return _round(sum(items) / len(items))


def _round(value: float) -> float:
    return round(value, 6)


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Generate a model disagreement report.")
    parser.add_argument("input", help="JSON or JSONL model score rows")
    parser.add_argument("output", help="Path to write report JSON")
    args = parser.parse_args(argv)

    report = generate_disagreement_report(load_score_rows(args.input))
    dump_report(report, args.output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
