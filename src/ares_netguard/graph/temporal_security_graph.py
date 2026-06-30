"""Deterministic privacy-safe temporal security graph novelty evidence.

The v0 adapter is a stdlib-only graph novelty baseline over synthetic,
coarse-grained edge observations. It does not perform packet capture, resolve
identifiers, call enrichment APIs, store raw indicators, or claim graph ML/GNN
behavior.
"""

from __future__ import annotations

import argparse
import ipaddress
import json
import math
import re
from collections import defaultdict
from collections.abc import Mapping, Sequence
from datetime import datetime
from pathlib import Path
from typing import Any

from ares_netguard.models.disagreement import ROW_SCHEMA_VERSION

REPORT_SCHEMA_VERSION = "temporal_security_graph_report.v0"
INPUT_ROW_SCHEMA_VERSION = "temporal_graph_edge_row.v0"
MODEL_ID = "graph_novelty"
MODEL_FAMILY = "experimental_graph"
DEFAULT_HISTORY_WINDOW = 3
DEFAULT_MIN_HISTORY_WINDOWS = 2

REQUIRED_INPUT_FIELDS = frozenset(
    {
        "schema_version",
        "event_id",
        "entity_id",
        "window_start",
        "source_node_type",
        "source_node_id",
        "edge_type",
        "target_node_type",
        "target_node_id",
    }
)
OPTIONAL_INPUT_FIELDS = frozenset({"observed_count"})
ALLOWED_INPUT_FIELDS = REQUIRED_INPUT_FIELDS | OPTIONAL_INPUT_FIELDS

ALLOWED_NODE_TYPES = frozenset(
    {
        "host",
        "process",
        "user",
        "domain",
        "ip",
        "asn",
        "alert",
        "model_signal",
        "file",
        "service",
    }
)
ALLOWED_EDGE_TYPES = frozenset(
    {
        "connected_to",
        "resolved",
        "spawned",
        "triggered",
        "co_occurred",
        "authenticated_to",
        "downloaded",
        "wrote_file",
        "shares_destination",
        "communicated_with",
    }
)
SYMMETRIC_EDGE_TYPES = frozenset({"co_occurred", "shares_destination"})

GRAPH_EVIDENCE_ROW_FIELDS = frozenset(
    {
        "event_id",
        "entity_id",
        "window_start",
        "source_node_type",
        "source_node_id",
        "edge_type",
        "target_node_type",
        "target_node_id",
        "canonical_edge_key",
        "observed_count",
        "history_window_count",
        "warmup",
        "historical_edge_window_count",
        "historical_neighbor_count",
        "current_neighbor_count",
        "historical_degree_mean",
        "current_degree",
        "rare_edge_score",
        "new_neighbor_ratio",
        "degree_change_score",
        "graph_novelty_risk",
        "model_id",
        "model_family",
    }
)

JsonMap = dict[str, Any]
NodeKey = tuple[str, str]
NeighborKey = tuple[str, str, str]

SAFE_EVENT_ID_RE = re.compile(r"^(?:edge|event|fixture)-[a-z0-9][a-z0-9_-]{0,62}$")
SAFE_ENTITY_ID_RE = re.compile(r"^(?:asset|entity|fixture|host|sensor)-[a-z0-9][a-z0-9_-]{0,62}$")
SAFE_NODE_ID_RE = re.compile(r"^[a-z][a-z0-9_]*-[a-z0-9][a-z0-9_-]{0,62}$")
EDGE_KEY_RE = re.compile(
    r"^[a-z_]+:[a-z][a-z0-9_]*-[a-z0-9][a-z0-9_-]{0,62}"
    r"\|[a-z_]+\|"
    r"[a-z_]+:[a-z][a-z0-9_]*-[a-z0-9][a-z0-9_-]{0,62}$"
)
URL_RE = re.compile(r"(?i)\b(?:https?|ftp)://")
EMAIL_RE = re.compile(r"(?i)\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b")
IPV4_RE = re.compile(r"\b(?:\d{1,3}\.){3}\d{1,3}\b")
DOMAIN_RE = re.compile(
    r"(?i)\b[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?"
    r"(?:\.[a-z](?:[a-z0-9-]{0,61}[a-z0-9])?)+\b"
)
PATH_RE = re.compile(r"(?i)(?:^|[\s=])(?:/[a-z0-9._-]+){2,}|\b[a-z]:\\")
SECRET_RE = re.compile(r"(?i)\b(?:password|passwd|credential|secret|token|api[_-]?key)\b")
COMMAND_LINE_RE = re.compile(
    r"(?i)(?:^|\s)(?:bash|sh|cmd(?:\.exe)?|powershell|pwsh|curl|wget)\s"
    r"|[;&|]{2}|`|(?:^|\s)-{1,2}[a-z][\w-]*"
)

FORBIDDEN_KEY_PARTS = (
    "ip",
    "domain",
    "hostname",
    "host_name",
    "url",
    "uri",
    "payload",
    "packet",
    "pcap",
    "credential",
    "password",
    "passwd",
    "secret",
    "token",
    "username",
    "user_name",
    "email",
    "filepath",
    "file_path",
    "path",
    "command",
    "cmdline",
    "cmd_line",
)

NODE_ID_PREFIXES: Mapping[str, tuple[str, ...]] = {
    "host": ("host-",),
    "process": ("process-",),
    "user": ("user-",),
    "domain": ("domain-",),
    "ip": ("ip-",),
    "asn": ("asn-",),
    "alert": ("alert-",),
    "model_signal": ("model-signal-", "model_signal-"),
    "file": ("file-",),
    "service": ("service-",),
}


def load_temporal_graph_edge_rows(path: str | Path) -> list[JsonMap]:
    """Load JSON or JSONL temporal_graph_edge_row.v0 rows with strict JSON parsing."""
    source = Path(path)
    text = source.read_text(encoding="utf-8").strip()
    if not text:
        return []

    if source.suffix == ".jsonl":
        return [_loads_strict(line) for line in text.splitlines() if line.strip()]

    payload = _loads_strict(text)
    if isinstance(payload, list):
        return payload
    if isinstance(payload, dict) and isinstance(payload.get("rows"), list):
        return payload["rows"]
    if isinstance(payload, dict):
        return [payload]
    raise ValueError(f"unsupported temporal graph edge payload in {source}")


def generate_temporal_security_graph_report(
    rows: Sequence[Mapping[str, Any]],
    *,
    history_window: int = DEFAULT_HISTORY_WINDOW,
    min_history_windows: int = DEFAULT_MIN_HISTORY_WINDOWS,
) -> JsonMap:
    """Generate deterministic temporal graph novelty evidence from edge rows."""
    _validate_settings(history_window, min_history_windows)
    normalized = [_normalize_input_row(row) for row in rows]
    _validate_unique_event_ids(normalized)
    normalized = sorted(
        normalized,
        key=lambda item: (item["entity_id"], item["timestamp"], item["event_id"]),
    )

    rows_by_entity: dict[str, list[JsonMap]] = defaultdict(list)
    for row in normalized:
        rows_by_entity[row["entity_id"]].append(row)

    evidence_rows: list[JsonMap] = []
    for entity_id in sorted(rows_by_entity):
        history: list[JsonMap] = []
        for window_rows in _entity_windows(rows_by_entity[entity_id]):
            current_snapshot = _snapshot_from_rows(window_rows)
            warmup = len(history) < min_history_windows
            for row in sorted(window_rows, key=lambda item: item["event_id"]):
                evidence = _build_evidence_row(row, current_snapshot, history, warmup=warmup)
                validate_temporal_graph_evidence_row(evidence)
                evidence_rows.append(evidence)

            history.append(current_snapshot)
            if len(history) > history_window:
                history = history[-history_window:]

    evidence_rows = sorted(
        evidence_rows,
        key=lambda item: (item["entity_id"], item["window_start"], item["event_id"]),
    )
    return {
        "schema_version": REPORT_SCHEMA_VERSION,
        "model_id": MODEL_ID,
        "model_family": MODEL_FAMILY,
        "history_window": history_window,
        "min_history_windows": min_history_windows,
        "edge_observation_count": len(normalized),
        "rows": evidence_rows,
    }


def temporal_graph_evidence_to_score_rows(
    report_or_rows: Mapping[str, Any] | Sequence[Mapping[str, Any]],
) -> list[JsonMap]:
    """Convert graph evidence rows into model_score_row.v0 rows.

    Multiple edge evidence rows for the same entity/window are folded into one
    graph_novelty score using the maximum graph novelty risk while preserving
    sanitized edge-level evidence.
    """
    evidence_rows = _extract_graph_rows(report_or_rows)
    groups: dict[tuple[str, str], list[Mapping[str, Any]]] = defaultdict(list)

    for row in evidence_rows:
        validate_temporal_graph_evidence_row(row)
        groups[(row["entity_id"], row["window_start"])].append(row)

    score_rows: list[JsonMap] = []
    for entity_id, window_start in sorted(groups):
        rows_for_window = sorted(
            groups[(entity_id, window_start)],
            key=lambda item: item["event_id"],
        )
        risk = max(row["graph_novelty_risk"] for row in rows_for_window)
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
                        "evidence": [_score_evidence(row) for row in rows_for_window],
                    }
                },
            }
        )
    return score_rows


def validate_temporal_graph_evidence_row(row: Mapping[str, Any]) -> None:
    """Validate the strict v0 temporal graph evidence row schema."""
    if not isinstance(row, Mapping):
        raise ValueError("temporal graph evidence row must be an object")

    actual_fields = set(row)
    if actual_fields != GRAPH_EVIDENCE_ROW_FIELDS:
        missing = sorted(GRAPH_EVIDENCE_ROW_FIELDS - actual_fields)
        unexpected = sorted(actual_fields - GRAPH_EVIDENCE_ROW_FIELDS)
        details = []
        if missing:
            details.append(f"missing {missing}")
        if unexpected:
            details.append(f"unexpected {unexpected}")
        raise ValueError(f"temporal graph evidence row fields invalid: {', '.join(details)}")

    _required_safe_id(row, "event_id", SAFE_EVENT_ID_RE, "event")
    _required_safe_id(row, "entity_id", SAFE_ENTITY_ID_RE, "entity")
    _parse_window_start(_required_text(row, "window_start"))
    source_node_type = _node_type(row, "source_node_type")
    source_node_id = _required_safe_node_id(row, "source_node_id", source_node_type)
    edge_type = _edge_type(row, "edge_type")
    target_node_type = _node_type(row, "target_node_type")
    target_node_id = _required_safe_node_id(row, "target_node_id", target_node_type)

    canonical_edge_key = _required_text(row, "canonical_edge_key")
    canonical_source, canonical_target = _canonical_edge_nodes(
        edge_type,
        (source_node_type, source_node_id),
        (target_node_type, target_node_id),
    )
    if canonical_edge_key != _format_edge_key(edge_type, canonical_source, canonical_target):
        raise ValueError("canonical_edge_key does not match graph edge fields")
    if not EDGE_KEY_RE.fullmatch(canonical_edge_key):
        raise ValueError("canonical_edge_key must use sanitized graph node identifiers")

    _positive_int(row.get("observed_count"), "observed_count")
    _non_negative_int(row.get("history_window_count"), "history_window_count")
    _non_negative_int(
        row.get("historical_edge_window_count"),
        "historical_edge_window_count",
    )
    _non_negative_int(row.get("historical_neighbor_count"), "historical_neighbor_count")
    _non_negative_int(row.get("current_neighbor_count"), "current_neighbor_count")
    _non_negative_int(row.get("current_degree"), "current_degree")
    _bounded_number(row.get("historical_degree_mean"), "historical_degree_mean", 0.0, math.inf)
    _bounded_number(row.get("rare_edge_score"), "rare_edge_score", 0.0, 1.0)
    _bounded_number(row.get("new_neighbor_ratio"), "new_neighbor_ratio", 0.0, 1.0)
    _bounded_number(row.get("degree_change_score"), "degree_change_score", 0.0, 1.0)
    _bounded_number(row.get("graph_novelty_risk"), "graph_novelty_risk", 0.0, 1.0)

    if not isinstance(row["warmup"], bool):
        raise ValueError("warmup must be a boolean")
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


def _loads_strict(text: str) -> Any:
    def reject_constant(value: str) -> None:
        raise ValueError(f"non-strict JSON constant '{value}' is not allowed")

    return json.loads(text, parse_constant=reject_constant)


def _extract_graph_rows(
    report_or_rows: Mapping[str, Any] | Sequence[Mapping[str, Any]],
) -> Sequence[Mapping[str, Any]]:
    if isinstance(report_or_rows, Mapping):
        if report_or_rows.get("schema_version") != REPORT_SCHEMA_VERSION:
            raise ValueError(f"graph report requires schema_version '{REPORT_SCHEMA_VERSION}'")
        rows = report_or_rows.get("rows")
        if not isinstance(rows, list):
            raise ValueError("graph report requires a 'rows' list")
        return rows
    return report_or_rows


def _normalize_input_row(row: Mapping[str, Any]) -> JsonMap:
    if not isinstance(row, Mapping):
        raise ValueError("temporal graph edge row must be an object")

    _validate_allowed_fields(row)
    if row.get("schema_version") != INPUT_ROW_SCHEMA_VERSION:
        raise ValueError(
            f"temporal graph edge row requires schema_version '{INPUT_ROW_SCHEMA_VERSION}'"
        )

    event_id = _required_safe_id(row, "event_id", SAFE_EVENT_ID_RE, "event")
    entity_id = _required_safe_id(row, "entity_id", SAFE_ENTITY_ID_RE, "entity")
    window_start = _required_text(row, "window_start")
    timestamp = _parse_window_start(window_start)
    source_node_type = _node_type(row, "source_node_type")
    source_node_id = _required_safe_node_id(row, "source_node_id", source_node_type)
    edge_type = _edge_type(row, "edge_type")
    target_node_type = _node_type(row, "target_node_type")
    target_node_id = _required_safe_node_id(row, "target_node_id", target_node_type)
    observed_count = _positive_int(row.get("observed_count", 1), "observed_count")
    canonical_source, canonical_target = _canonical_edge_nodes(
        edge_type,
        (source_node_type, source_node_id),
        (target_node_type, target_node_id),
    )

    return {
        "event_id": event_id,
        "entity_id": entity_id,
        "window_start": window_start,
        "timestamp": timestamp,
        "source_node_type": source_node_type,
        "source_node_id": source_node_id,
        "edge_type": edge_type,
        "target_node_type": target_node_type,
        "target_node_id": target_node_id,
        "observed_count": observed_count,
        "canonical_source_node": canonical_source,
        "canonical_target_node": canonical_target,
        "canonical_edge_key": _format_edge_key(edge_type, canonical_source, canonical_target),
        "canonical_neighbor_key": (edge_type, *canonical_target),
    }


def _validate_allowed_fields(row: Mapping[str, Any]) -> None:
    for key in row:
        if not isinstance(key, str):
            raise ValueError("temporal graph edge row keys must be strings")
        lowered = key.lower()
        if key not in ALLOWED_INPUT_FIELDS and any(part in lowered for part in FORBIDDEN_KEY_PARTS):
            raise ValueError(f"unsafe raw field '{key}' is not allowed")

    actual_fields = set(row)
    missing = sorted(REQUIRED_INPUT_FIELDS - actual_fields)
    if missing:
        raise ValueError(f"temporal graph edge row missing required fields {missing}")

    unexpected = sorted(actual_fields - ALLOWED_INPUT_FIELDS)
    if unexpected:
        raise ValueError(f"temporal graph edge row has unexpected fields {unexpected}")


def _validate_unique_event_ids(rows: Sequence[Mapping[str, Any]]) -> None:
    seen_event_ids: set[str] = set()
    for row in rows:
        event_id = row["event_id"]
        if event_id in seen_event_ids:
            raise ValueError("duplicate event_id")
        seen_event_ids.add(event_id)


def _entity_windows(rows: Sequence[Mapping[str, Any]]) -> list[list[Mapping[str, Any]]]:
    windows: dict[tuple[datetime, str], list[Mapping[str, Any]]] = defaultdict(list)
    for row in rows:
        windows[(row["timestamp"], row["window_start"])].append(row)
    return [sorted(windows[key], key=lambda item: item["event_id"]) for key in sorted(windows)]


def _snapshot_from_rows(rows: Sequence[Mapping[str, Any]]) -> JsonMap:
    edge_keys = {row["canonical_edge_key"] for row in rows}
    neighbors_by_source: dict[NodeKey, set[NeighborKey]] = defaultdict(set)

    for row in rows:
        source = row["canonical_source_node"]
        neighbor = row["canonical_neighbor_key"]
        neighbors_by_source[source].add(neighbor)

    frozen_neighbors = {
        source: frozenset(neighbors) for source, neighbors in sorted(neighbors_by_source.items())
    }
    return {
        "edge_keys": frozenset(edge_keys),
        "neighbors_by_source": frozen_neighbors,
        "degree_by_source": {
            source: len(neighbors) for source, neighbors in sorted(frozen_neighbors.items())
        },
    }


def _build_evidence_row(
    row: Mapping[str, Any],
    current_snapshot: Mapping[str, Any],
    history: Sequence[Mapping[str, Any]],
    *,
    warmup: bool,
) -> JsonMap:
    history_window_count = len(history)
    source = row["canonical_source_node"]
    current_neighbors = current_snapshot["neighbors_by_source"].get(source, frozenset())
    historical_neighbors = _historical_neighbors(history, source)
    current_degree = current_snapshot["degree_by_source"].get(source, 0)
    historical_degrees = [snapshot["degree_by_source"].get(source, 0) for snapshot in history]
    historical_degree_mean = _mean(historical_degrees) if historical_degrees else 0.0
    historical_edge_window_count = sum(
        1 for snapshot in history if row["canonical_edge_key"] in snapshot["edge_keys"]
    )

    if warmup:
        rare_edge_score = 0.0
        new_neighbor_ratio = 0.0
        degree_change_score = 0.0
    else:
        rare_edge_score = 1.0 - (historical_edge_window_count / history_window_count)
        new_neighbor_ratio = _new_neighbor_ratio(current_neighbors, historical_neighbors)
        degree_change_score = _degree_change_score(current_degree, historical_degree_mean)

    graph_novelty_risk = max(rare_edge_score, new_neighbor_ratio, degree_change_score)

    return {
        "event_id": row["event_id"],
        "entity_id": row["entity_id"],
        "window_start": row["window_start"],
        "source_node_type": row["source_node_type"],
        "source_node_id": row["source_node_id"],
        "edge_type": row["edge_type"],
        "target_node_type": row["target_node_type"],
        "target_node_id": row["target_node_id"],
        "canonical_edge_key": row["canonical_edge_key"],
        "observed_count": row["observed_count"],
        "history_window_count": history_window_count,
        "warmup": warmup,
        "historical_edge_window_count": historical_edge_window_count,
        "historical_neighbor_count": len(historical_neighbors),
        "current_neighbor_count": len(current_neighbors),
        "historical_degree_mean": _round(historical_degree_mean),
        "current_degree": current_degree,
        "rare_edge_score": _round(rare_edge_score),
        "new_neighbor_ratio": _round(new_neighbor_ratio),
        "degree_change_score": _round(degree_change_score),
        "graph_novelty_risk": _round(graph_novelty_risk),
        "model_id": MODEL_ID,
        "model_family": MODEL_FAMILY,
    }


def _historical_neighbors(
    history: Sequence[Mapping[str, Any]],
    source: NodeKey,
) -> frozenset[NeighborKey]:
    neighbors: set[NeighborKey] = set()
    for snapshot in history:
        neighbors.update(snapshot["neighbors_by_source"].get(source, frozenset()))
    return frozenset(neighbors)


def _new_neighbor_ratio(
    current_neighbors: Sequence[NeighborKey],
    historical_neighbors: Sequence[NeighborKey],
) -> float:
    if not current_neighbors:
        return 0.0
    historical = set(historical_neighbors)
    new_neighbors = [neighbor for neighbor in current_neighbors if neighbor not in historical]
    return len(new_neighbors) / len(current_neighbors)


def _degree_change_score(current_degree: int, historical_degree_mean: float) -> float:
    if current_degree <= historical_degree_mean:
        return 0.0
    return (current_degree - historical_degree_mean) / max(float(current_degree), 1.0)


def _score_evidence(row: Mapping[str, Any]) -> JsonMap:
    return {
        "event_id": row["event_id"],
        "canonical_edge_key": row["canonical_edge_key"],
        "source_node_type": row["source_node_type"],
        "source_node_id": row["source_node_id"],
        "edge_type": row["edge_type"],
        "target_node_type": row["target_node_type"],
        "target_node_id": row["target_node_id"],
        "observed_count": row["observed_count"],
        "history_window_count": row["history_window_count"],
        "warmup": row["warmup"],
        "rare_edge_score": row["rare_edge_score"],
        "new_neighbor_ratio": row["new_neighbor_ratio"],
        "degree_change_score": row["degree_change_score"],
        "graph_novelty_risk": row["graph_novelty_risk"],
    }


def _node_type(row: Mapping[str, Any], field: str) -> str:
    raw_value = row.get(field)
    if not isinstance(raw_value, str):
        raise ValueError(f"{field} must be a string")
    value = raw_value.strip().lower().replace("-", "_").replace(" ", "_")
    _reject_unsafe_text(value, field)
    if value not in ALLOWED_NODE_TYPES:
        raise ValueError(f"{field} has unknown node type '{raw_value}'")
    return value


def _edge_type(row: Mapping[str, Any], field: str) -> str:
    raw_value = row.get(field)
    if not isinstance(raw_value, str):
        raise ValueError(f"{field} must be a string")
    value = raw_value.strip().lower().replace("-", "_").replace(" ", "_")
    _reject_unsafe_text(value, field)
    if value not in ALLOWED_EDGE_TYPES:
        raise ValueError(f"{field} has unknown edge type '{raw_value}'")
    return value


def _canonical_edge_nodes(
    edge_type: str,
    source: NodeKey,
    target: NodeKey,
) -> tuple[NodeKey, NodeKey]:
    if edge_type in SYMMETRIC_EDGE_TYPES and target < source:
        return target, source
    return source, target


def _format_edge_key(edge_type: str, source: NodeKey, target: NodeKey) -> str:
    return f"{source[0]}:{source[1]}|{edge_type}|{target[0]}:{target[1]}"


def _required_text(row: Mapping[str, Any], key: str) -> str:
    value = row.get(key)
    if not isinstance(value, str) or not value.strip():
        raise ValueError(f"temporal graph edge row requires non-empty '{key}'")
    cleaned = value.strip()
    _reject_unsafe_text(cleaned, key)
    return cleaned


def _required_safe_id(
    row: Mapping[str, Any],
    key: str,
    pattern: re.Pattern[str],
    kind: str,
) -> str:
    value = _required_text(row, key)
    if not pattern.fullmatch(value):
        raise ValueError(
            f"{key} must be a synthetic/coarse {kind} identifier, "
            "not a raw username, hostname, address, or private identifier"
        )
    return value


def _required_safe_node_id(row: Mapping[str, Any], key: str, node_type: str) -> str:
    value = _required_text(row, key)
    if not SAFE_NODE_ID_RE.fullmatch(value):
        raise ValueError(
            f"{key} must be a synthetic/coarse {node_type} node identifier, "
            "not a raw username, hostname, address, path, or private identifier"
        )
    allowed_prefixes = NODE_ID_PREFIXES[node_type]
    if not value.startswith(allowed_prefixes):
        raise ValueError(f"{key} must use a synthetic prefix for node type '{node_type}'")
    return value


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
        cleaned = candidate.strip("[](){}<>")
        if not cleaned:
            continue
        try:
            ipaddress.ip_address(cleaned)
        except ValueError:
            continue
        return True
    return False


def _parse_window_start(value: str) -> datetime:
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError as exc:
        raise ValueError("window_start must be an ISO-8601 timestamp") from exc
    if parsed.tzinfo is None:
        raise ValueError("window_start must include timezone information")
    return parsed


def _validate_settings(history_window: int, min_history_windows: int) -> None:
    if (
        isinstance(history_window, bool)
        or not isinstance(history_window, int)
        or history_window < 1
    ):
        raise ValueError("history_window must be a positive integer")
    if (
        isinstance(min_history_windows, bool)
        or not isinstance(min_history_windows, int)
        or min_history_windows < 1
    ):
        raise ValueError("min_history_windows must be a positive integer")
    if min_history_windows > history_window:
        raise ValueError("min_history_windows must be less than or equal to history_window")


def _positive_int(raw_value: Any, field: str) -> int:
    value = _non_negative_int(raw_value, field)
    if value < 1:
        raise ValueError(f"{field} must be positive")
    return value


def _non_negative_int(raw_value: Any, field: str) -> int:
    if isinstance(raw_value, bool) or not isinstance(raw_value, int):
        raise ValueError(f"{field} must be a non-negative integer")
    if raw_value < 0:
        raise ValueError(f"{field} must be a non-negative integer")
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


def _mean(values: Sequence[float]) -> float:
    return sum(values) / len(values)


def _round(value: float) -> float:
    rounded = round(value, 6)
    return 0.0 if rounded == 0 else rounded


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Generate deterministic temporal security graph novelty evidence."
    )
    parser.add_argument("input", help="JSON or JSONL temporal_graph_edge_row.v0 rows")
    parser.add_argument("output", help="Path to write temporal graph report JSON")
    parser.add_argument(
        "--history-window",
        type=int,
        default=DEFAULT_HISTORY_WINDOW,
        help="Number of previous windows used as bounded graph history",
    )
    parser.add_argument(
        "--min-history-windows",
        type=int,
        default=DEFAULT_MIN_HISTORY_WINDOWS,
        help="Minimum previous windows before novelty scores are emitted",
    )
    args = parser.parse_args(argv)

    report = generate_temporal_security_graph_report(
        load_temporal_graph_edge_rows(args.input),
        history_window=args.history_window,
        min_history_windows=args.min_history_windows,
    )
    dump_report(report, args.output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
