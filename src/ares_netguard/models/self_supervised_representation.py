"""Privacy-safe traffic representation evidence producer.

The v0 adapter is a deterministic stdlib-only proxy for self-supervised traffic
representation evidence. It does not train a model, download weights, inspect
payloads, store raw identifiers, or claim ET-BERT/MAE/contrastive behavior.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import re
from collections import Counter, defaultdict
from collections.abc import Mapping, Sequence
from datetime import datetime
from pathlib import Path
from typing import Any

from ares_netguard.models.disagreement import ROW_SCHEMA_VERSION

REPORT_SCHEMA_VERSION = "traffic_representation_report.v0"
INPUT_ROW_SCHEMA_VERSION = "traffic_sequence_row.v0"
MODEL_ID = "self_supervised_representation"
MODEL_FAMILY = "experimental_self_supervised"
DEFAULT_EMBEDDING_DIMENSIONS = 16

REQUIRED_INPUT_FIELDS = frozenset(
    {
        "schema_version",
        "sequence_id",
        "entity_id",
        "window_start",
    }
)
OPTIONAL_INPUT_FIELDS = frozenset(
    {
        "protocol",
        "direction",
        "service",
        "destination_port",
        "bytes_total",
        "duration_ms",
        "tcp_flags",
        "tls_version",
        "dns_outcome",
        "payload_entropy",
    }
)
ALLOWED_INPUT_FIELDS = REQUIRED_INPUT_FIELDS | OPTIONAL_INPUT_FIELDS

TOKEN_FIELD_ORDER = (
    "protocol",
    "direction",
    "service",
    "port_bucket",
    "bytes_bucket",
    "duration_bucket",
    "tcp_flag_category",
    "tls_version_class",
    "dns_outcome_class",
    "entropy_bucket",
)

REPRESENTATION_ROW_FIELDS = frozenset(
    {
        "sequence_id",
        "entity_id",
        "window_start",
        "tokens",
        "token_count",
        "embedding",
        "embedding_dimensions",
        "embedding_novelty_score",
        "rare_token_count",
        "representation_risk",
        "model_id",
        "model_family",
    }
)

JsonMap = dict[str, Any]

SAFE_ENTITY_ID_RE = re.compile(r"^(?:asset|entity|fixture|host|sensor)-[a-z0-9][a-z0-9_-]{0,62}$")
SAFE_SEQUENCE_ID_RE = re.compile(r"^(?:fixture|seq|sequence)-[a-z0-9][a-z0-9_-]{0,62}$")
TOKEN_RE = re.compile(r"^[a-z0-9_]+:[a-z0-9_]+$")
URL_RE = re.compile(r"(?i)\b(?:https?|ftp)://")
EMAIL_RE = re.compile(r"(?i)\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b")
IPV4_RE = re.compile(r"\b(?:\d{1,3}\.){3}\d{1,3}\b")
DOMAIN_RE = re.compile(
    r"(?i)\b[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?"
    r"(?:\.[a-z](?:[a-z0-9-]{0,61}[a-z0-9])?)+\b"
)
PATH_RE = re.compile(r"(?i)(?:^|[\s=])(?:/[a-z0-9._-]+){2,}|\b[a-z]:\\")
SECRET_RE = re.compile(r"(?i)\b(?:password|passwd|credential|secret|token|api[_-]?key)\b")

FORBIDDEN_KEY_PARTS = (
    "ip",
    "domain",
    "host",
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
    "filepath",
    "file_path",
    "path",
    "email",
)

PROTOCOL_ALIASES = {
    "tcp": "tcp",
    "udp": "udp",
    "icmp": "icmp",
    "icmpv6": "icmp",
    "other": "other",
    "unknown": "unknown",
}
DIRECTION_ALIASES = {
    "internal_to_external": "internal_to_external",
    "outbound": "internal_to_external",
    "egress": "internal_to_external",
    "external_to_internal": "external_to_internal",
    "inbound": "external_to_internal",
    "ingress": "external_to_internal",
    "internal": "internal",
    "lateral": "internal",
    "internal_to_internal": "internal",
    "external": "external",
    "unknown": "unknown",
}
SERVICE_ALIASES = {
    "dns": "dns",
    "http": "http",
    "https": "https",
    "tls": "tls",
    "ssh": "ssh",
    "smtp": "smtp",
    "ntp": "ntp",
    "rdp": "rdp",
    "other": "other",
    "unknown": "unknown",
}
TCP_FLAG_ALIASES = {
    "none": "none",
    "syn": "syn",
    "syn_ack": "syn_ack",
    "syn-ack": "syn_ack",
    "sa": "syn_ack",
    "established": "established",
    "ack": "established",
    "fin": "fin",
    "rst": "reset",
    "reset": "reset",
    "other": "other",
    "unknown": "unknown",
}
TLS_VERSION_ALIASES = {
    "none": "no_tls",
    "not_tls": "no_tls",
    "no_tls": "no_tls",
    "ssl": "legacy",
    "ssl3": "legacy",
    "tls1.0": "legacy",
    "tlsv1.0": "legacy",
    "1.0": "legacy",
    "tls1.1": "legacy",
    "tlsv1.1": "legacy",
    "1.1": "legacy",
    "tls1.2": "tls12",
    "tlsv1.2": "tls12",
    "1.2": "tls12",
    "tls1.3": "tls13",
    "tlsv1.3": "tls13",
    "1.3": "tls13",
    "other": "other",
    "unknown": "unknown",
}
DNS_OUTCOME_ALIASES = {
    "none": "not_dns",
    "not_dns": "not_dns",
    "success": "success",
    "ok": "success",
    "noerror": "success",
    "nxdomain": "nxdomain",
    "servfail": "servfail",
    "refused": "refused",
    "timeout": "timeout",
    "other": "other",
    "unknown": "unknown",
}


def load_traffic_sequence_rows(path: str | Path) -> list[JsonMap]:
    """Load JSON or JSONL traffic_sequence_row.v0 rows with strict JSON parsing."""
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
    raise ValueError(f"unsupported traffic sequence payload in {source}")


def tokenize_sequence_row(row: Mapping[str, Any]) -> list[str]:
    """Return only allowlisted coarse metadata tokens for one traffic sequence."""
    return _tokens_from_normalized(_normalize_input_row(row))


def generate_representation_report(
    rows: Sequence[Mapping[str, Any]],
    *,
    embedding_dimensions: int = DEFAULT_EMBEDDING_DIMENSIONS,
) -> JsonMap:
    """Generate deterministic embedding-style evidence from sanitized metadata rows."""
    _validate_embedding_dimensions(embedding_dimensions)
    normalized = [_normalize_input_row(row) for row in rows]
    _validate_input_order(normalized)

    tokenized_rows = [
        {
            "sequence_id": row["sequence_id"],
            "entity_id": row["entity_id"],
            "window_start": row["window_start"],
            "tokens": _tokens_from_normalized(row),
        }
        for row in normalized
    ]
    document_frequency = _token_document_frequency(item["tokens"] for item in tokenized_rows)
    sequence_count = len(tokenized_rows)

    evidence_rows: list[JsonMap] = []
    for item in tokenized_rows:
        tokens = item["tokens"]
        novelty_score = _token_novelty_score(tokens, document_frequency, sequence_count)
        evidence = {
            "sequence_id": item["sequence_id"],
            "entity_id": item["entity_id"],
            "window_start": item["window_start"],
            "tokens": tokens,
            "token_count": len(tokens),
            "embedding": _embedding_from_tokens(tokens, embedding_dimensions),
            "embedding_dimensions": embedding_dimensions,
            "embedding_novelty_score": novelty_score,
            "rare_token_count": sum(1 for token in tokens if document_frequency[token] == 1),
            "representation_risk": novelty_score,
            "model_id": MODEL_ID,
            "model_family": MODEL_FAMILY,
        }
        validate_representation_evidence_row(evidence)
        evidence_rows.append(evidence)

    evidence_rows = sorted(
        evidence_rows,
        key=lambda item: (item["entity_id"], item["window_start"], item["sequence_id"]),
    )
    return {
        "schema_version": REPORT_SCHEMA_VERSION,
        "model_id": MODEL_ID,
        "model_family": MODEL_FAMILY,
        "embedding_dimensions": embedding_dimensions,
        "sequence_count": sequence_count,
        "token_field_order": list(TOKEN_FIELD_ORDER),
        "rows": evidence_rows,
    }


def representation_evidence_to_score_rows(
    report_or_rows: Mapping[str, Any] | Sequence[Mapping[str, Any]],
) -> list[JsonMap]:
    """Convert representation evidence rows into model_score_row.v0 rows.

    Multiple sequence rows for the same entity/window are folded into one score
    using the maximum representation risk while preserving sanitized evidence.
    """
    evidence_rows = _extract_representation_rows(report_or_rows)
    groups: dict[tuple[str, str], list[Mapping[str, Any]]] = defaultdict(list)

    for row in evidence_rows:
        validate_representation_evidence_row(row)
        groups[(row["entity_id"], row["window_start"])].append(row)

    score_rows: list[JsonMap] = []
    for (entity_id, window_start), rows_for_window in sorted(groups.items()):
        evidence = sorted(rows_for_window, key=lambda item: item["sequence_id"])
        risk = max(row["representation_risk"] for row in evidence)
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
                        "evidence": [_score_evidence(row) for row in evidence],
                    }
                },
            }
        )
    return score_rows


def validate_representation_evidence_row(row: Mapping[str, Any]) -> None:
    """Validate the strict v0 representation evidence row schema."""
    if not isinstance(row, Mapping):
        raise ValueError("representation evidence row must be an object")

    actual_fields = set(row)
    if actual_fields != REPRESENTATION_ROW_FIELDS:
        missing = sorted(REPRESENTATION_ROW_FIELDS - actual_fields)
        unexpected = sorted(actual_fields - REPRESENTATION_ROW_FIELDS)
        details = []
        if missing:
            details.append(f"missing {missing}")
        if unexpected:
            details.append(f"unexpected {unexpected}")
        raise ValueError(f"representation evidence row fields invalid: {', '.join(details)}")

    _required_safe_id(row, "sequence_id", SAFE_SEQUENCE_ID_RE, "sequence")
    _required_safe_id(row, "entity_id", SAFE_ENTITY_ID_RE, "entity")
    _parse_window_start(_required_text(row, "window_start"))

    tokens = row.get("tokens")
    if not isinstance(tokens, list) or not tokens:
        raise ValueError("tokens must be a non-empty list")
    for token in tokens:
        _validate_token(token)

    token_count = _positive_int(row.get("token_count"), "token_count")
    if token_count != len(tokens):
        raise ValueError("token_count must match tokens length")

    embedding_dimensions = _positive_int(row.get("embedding_dimensions"), "embedding_dimensions")
    embedding = row.get("embedding")
    if not isinstance(embedding, list) or len(embedding) != embedding_dimensions:
        raise ValueError("embedding must be a list matching embedding_dimensions")
    for value in embedding:
        _bounded_number(value, "embedding value", -1.0, 1.0)

    _bounded_number(row.get("embedding_novelty_score"), "embedding_novelty_score", 0.0, 1.0)
    _bounded_number(row.get("representation_risk"), "representation_risk", 0.0, 1.0)

    rare_token_count = _non_negative_int(row.get("rare_token_count"), "rare_token_count")
    if rare_token_count > token_count:
        raise ValueError("rare_token_count must be less than or equal to token_count")

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


def _extract_representation_rows(
    report_or_rows: Mapping[str, Any] | Sequence[Mapping[str, Any]],
) -> Sequence[Mapping[str, Any]]:
    if isinstance(report_or_rows, Mapping):
        if report_or_rows.get("schema_version") != REPORT_SCHEMA_VERSION:
            raise ValueError(
                f"representation report requires schema_version '{REPORT_SCHEMA_VERSION}'"
            )
        rows = report_or_rows.get("rows")
        if not isinstance(rows, list):
            raise ValueError("representation report requires a 'rows' list")
        return rows
    return report_or_rows


def _normalize_input_row(row: Mapping[str, Any]) -> JsonMap:
    if not isinstance(row, Mapping):
        raise ValueError("traffic sequence row must be an object")

    _validate_allowed_fields(row)
    if row.get("schema_version") != INPUT_ROW_SCHEMA_VERSION:
        raise ValueError(
            f"traffic sequence row requires schema_version '{INPUT_ROW_SCHEMA_VERSION}'"
        )

    sequence_id = _required_safe_id(row, "sequence_id", SAFE_SEQUENCE_ID_RE, "sequence")
    entity_id = _required_safe_id(row, "entity_id", SAFE_ENTITY_ID_RE, "entity")
    window_start = _required_text(row, "window_start")
    timestamp = _parse_window_start(window_start)

    return {
        "sequence_id": sequence_id,
        "entity_id": entity_id,
        "window_start": window_start,
        "timestamp": timestamp,
        "protocol": _choice_value(row, "protocol", PROTOCOL_ALIASES, "unknown"),
        "direction": _choice_value(row, "direction", DIRECTION_ALIASES, "unknown"),
        "service": _choice_value(row, "service", SERVICE_ALIASES, "unknown"),
        "port_bucket": _port_bucket(row.get("destination_port")),
        "bytes_bucket": _bytes_bucket(row.get("bytes_total")),
        "duration_bucket": _duration_bucket(row.get("duration_ms")),
        "tcp_flag_category": _choice_value(row, "tcp_flags", TCP_FLAG_ALIASES, "none"),
        "tls_version_class": _choice_value(row, "tls_version", TLS_VERSION_ALIASES, "no_tls"),
        "dns_outcome_class": _choice_value(row, "dns_outcome", DNS_OUTCOME_ALIASES, "not_dns"),
        "entropy_bucket": _entropy_bucket(row.get("payload_entropy")),
    }


def _validate_allowed_fields(row: Mapping[str, Any]) -> None:
    for key in row:
        if not isinstance(key, str):
            raise ValueError("traffic sequence row keys must be strings")
        lowered = key.lower()
        if key not in ALLOWED_INPUT_FIELDS and any(part in lowered for part in FORBIDDEN_KEY_PARTS):
            raise ValueError(f"unsafe raw field '{key}' is not allowed")

    actual_fields = set(row)
    missing = sorted(REQUIRED_INPUT_FIELDS - actual_fields)
    if missing:
        raise ValueError(f"traffic sequence row missing required fields {missing}")

    unexpected = sorted(actual_fields - ALLOWED_INPUT_FIELDS)
    if unexpected:
        raise ValueError(f"traffic sequence row has unexpected fields {unexpected}")


def _validate_input_order(rows: Sequence[Mapping[str, Any]]) -> None:
    seen_sequence_ids: set[str] = set()
    last_timestamp_by_entity: dict[str, datetime] = {}

    for row in rows:
        sequence_id = row["sequence_id"]
        if sequence_id in seen_sequence_ids:
            raise ValueError("duplicate sequence_id")
        seen_sequence_ids.add(sequence_id)

        entity_id = row["entity_id"]
        previous = last_timestamp_by_entity.get(entity_id)
        if previous is not None and row["timestamp"] <= previous:
            raise ValueError("traffic sequence rows must be strictly increasing per entity_id")
        last_timestamp_by_entity[entity_id] = row["timestamp"]


def _tokens_from_normalized(row: Mapping[str, Any]) -> list[str]:
    tokens = [f"{field}:{row[field]}" for field in TOKEN_FIELD_ORDER]
    for token in tokens:
        _validate_token(token)
    return tokens


def _token_document_frequency(token_rows: Sequence[Sequence[str]]) -> Counter[str]:
    frequency: Counter[str] = Counter()
    for tokens in token_rows:
        frequency.update(set(tokens))
    return frequency


def _token_novelty_score(
    tokens: Sequence[str],
    document_frequency: Counter[str],
    sequence_count: int,
) -> float:
    if not tokens or sequence_count < 1:
        return 0.0
    novelty = sum(1.0 - (document_frequency[token] / sequence_count) for token in tokens)
    return _round(novelty / len(tokens))


def _embedding_from_tokens(tokens: Sequence[str], dimensions: int) -> list[float]:
    vector = [0.0] * dimensions
    for token in sorted(tokens):
        digest = hashlib.sha256(token.encode("utf-8")).digest()
        index = int.from_bytes(digest[:4], "big") % dimensions
        sign = 1.0 if digest[4] % 2 == 0 else -1.0
        vector[index] += sign

    scale = max(len(tokens), 1)
    return [_round(value / scale) for value in vector]


def _score_evidence(row: Mapping[str, Any]) -> JsonMap:
    return {
        "sequence_id": row["sequence_id"],
        "tokens": list(row["tokens"]),
        "token_count": row["token_count"],
        "embedding_dimensions": row["embedding_dimensions"],
        "embedding_novelty_score": row["embedding_novelty_score"],
        "rare_token_count": row["rare_token_count"],
        "representation_risk": row["representation_risk"],
    }


def _choice_value(
    row: Mapping[str, Any],
    field: str,
    aliases: Mapping[str, str],
    missing_value: str,
) -> str:
    raw_value = row.get(field)
    if raw_value is None:
        return missing_value
    if not isinstance(raw_value, str):
        raise ValueError(f"{field} must be a string when provided")

    value = raw_value.strip().lower().replace(" ", "_")
    _reject_unsafe_text(value, field)
    return aliases.get(value, "other")


def _port_bucket(raw_value: Any) -> str:
    if raw_value is None:
        return "missing"
    value = _bounded_number(raw_value, "destination_port", 0.0, 65535.0)
    if not value.is_integer():
        raise ValueError("destination_port must be an integer")
    port = int(value)
    if port <= 1023:
        return "system"
    if port <= 49151:
        return "registered"
    return "dynamic"


def _bytes_bucket(raw_value: Any) -> str:
    if raw_value is None:
        return "missing"
    value = _bounded_number(raw_value, "bytes_total", 0.0, float("inf"))
    if value == 0:
        return "zero"
    if value <= 512:
        return "tiny"
    if value <= 4096:
        return "small"
    if value <= 65536:
        return "medium"
    if value <= 1048576:
        return "large"
    return "bulk"


def _duration_bucket(raw_value: Any) -> str:
    if raw_value is None:
        return "missing"
    value = _bounded_number(raw_value, "duration_ms", 0.0, float("inf"))
    if value < 1000:
        return "subsecond"
    if value < 30000:
        return "short"
    if value < 300000:
        return "medium"
    return "long"


def _entropy_bucket(raw_value: Any) -> str:
    if raw_value is None:
        return "missing"
    value = _bounded_number(raw_value, "payload_entropy", 0.0, 8.0)
    if value < 2.0:
        return "very_low"
    if value < 4.0:
        return "low"
    if value < 6.5:
        return "medium"
    return "high"


def _required_text(row: Mapping[str, Any], key: str) -> str:
    value = row.get(key)
    if not isinstance(value, str) or not value.strip():
        raise ValueError(f"traffic sequence row requires non-empty '{key}'")
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


def _reject_unsafe_text(value: str, field: str) -> None:
    if (
        URL_RE.search(value)
        or EMAIL_RE.search(value)
        or IPV4_RE.search(value)
        or DOMAIN_RE.search(value)
        or PATH_RE.search(value)
        or SECRET_RE.search(value)
    ):
        raise ValueError(f"{field} contains unsafe raw identifier content")


def _validate_token(token: Any) -> None:
    if not isinstance(token, str) or not TOKEN_RE.fullmatch(token):
        raise ValueError("tokens must use sanitized '<field>:<bucket>' values")
    _reject_unsafe_text(token, "token")


def _parse_window_start(value: str) -> datetime:
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError as exc:
        raise ValueError("window_start must be an ISO-8601 timestamp") from exc
    if parsed.tzinfo is None:
        raise ValueError("window_start must include timezone information")
    return parsed


def _validate_embedding_dimensions(value: int) -> None:
    if isinstance(value, bool) or not isinstance(value, int) or value < 4 or value > 256:
        raise ValueError("embedding_dimensions must be an integer between 4 and 256")


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


def _round(value: float) -> float:
    rounded = round(value, 6)
    return 0.0 if rounded == 0 else rounded


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Generate deterministic privacy-safe traffic representation evidence."
    )
    parser.add_argument("input", help="JSON or JSONL traffic_sequence_row.v0 rows")
    parser.add_argument("output", help="Path to write representation report JSON")
    parser.add_argument(
        "--embedding-dimensions",
        type=int,
        default=DEFAULT_EMBEDDING_DIMENSIONS,
        help="Fixed hash/count sketch width",
    )
    args = parser.parse_args(argv)

    report = generate_representation_report(
        load_traffic_sequence_rows(args.input),
        embedding_dimensions=args.embedding_dimensions,
    )
    dump_report(report, args.output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
