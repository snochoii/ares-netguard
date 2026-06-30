from __future__ import annotations

import json
from pathlib import Path

import pytest

from ares_netguard.models.self_supervised_representation import (
    MODEL_FAMILY,
    MODEL_ID,
    REPORT_SCHEMA_VERSION,
    dump_report,
    generate_representation_report,
    load_traffic_sequence_rows,
    representation_evidence_to_score_rows,
    tokenize_sequence_row,
    validate_representation_evidence_row,
)


def _sequence(
    sequence_id: str = "seq-001",
    window_start: str = "2026-01-01T00:00:00Z",
    *,
    entity_id: str = "asset-alpha",
) -> dict[str, object]:
    return {
        "schema_version": "traffic_sequence_row.v0",
        "sequence_id": sequence_id,
        "entity_id": entity_id,
        "window_start": window_start,
        "protocol": "tcp",
        "direction": "internal_to_external",
        "service": "https",
        "destination_port": 443,
        "bytes_total": 2400,
        "duration_ms": 850,
        "tcp_flags": "established",
        "tls_version": "1.3",
        "dns_outcome": "not_dns",
        "payload_entropy": 5.1,
    }


def test_tokenization_and_embedding_output_are_deterministic() -> None:
    sparse = {
        "schema_version": "traffic_sequence_row.v0",
        "sequence_id": "seq-sparse",
        "entity_id": "asset-beta",
        "window_start": "2026-01-01T00:00:00Z",
        "protocol": "icmp",
        "direction": "internal",
        "service": "unknown",
    }

    assert tokenize_sequence_row(_sequence()) == [
        "protocol:tcp",
        "direction:internal_to_external",
        "service:https",
        "port_bucket:system",
        "bytes_bucket:small",
        "duration_bucket:subsecond",
        "tcp_flag_category:established",
        "tls_version_class:tls13",
        "dns_outcome_class:not_dns",
        "entropy_bucket:medium",
    ]
    assert tokenize_sequence_row(sparse) == [
        "protocol:icmp",
        "direction:internal",
        "service:unknown",
        "port_bucket:missing",
        "bytes_bucket:missing",
        "duration_bucket:missing",
        "tcp_flag_category:none",
        "tls_version_class:no_tls",
        "dns_outcome_class:not_dns",
        "entropy_bucket:missing",
    ]

    rows = [
        _sequence("seq-001", "2026-01-01T00:00:00Z"),
        _sequence("seq-002", "2026-01-01T00:05:00Z"),
        sparse,
    ]
    report = generate_representation_report(rows)
    repeat_report = generate_representation_report(rows)

    assert report == repeat_report
    assert report["schema_version"] == REPORT_SCHEMA_VERSION
    assert report["embedding_dimensions"] == 16
    assert len(report["rows"][0]["embedding"]) == 16
    assert any(value != 0.0 for value in report["rows"][0]["embedding"])


def test_schema_validation_rejects_bad_rows() -> None:
    with pytest.raises(ValueError, match="traffic sequence row must be an object"):
        generate_representation_report([["not", "an", "object"]])  # type: ignore[list-item]

    missing = _sequence()
    del missing["sequence_id"]
    with pytest.raises(ValueError, match="missing required fields"):
        generate_representation_report([missing])

    with pytest.raises(ValueError, match="duplicate sequence_id"):
        generate_representation_report(
            [
                _sequence("seq-dup", "2026-01-01T00:00:00Z"),
                _sequence("seq-dup", "2026-01-01T00:05:00Z"),
            ]
        )

    with pytest.raises(ValueError, match="strictly increasing per entity_id"):
        generate_representation_report(
            [
                _sequence("seq-late", "2026-01-01T00:05:00Z"),
                _sequence("seq-early", "2026-01-01T00:00:00Z"),
            ]
        )


def test_privacy_guard_rejects_forbidden_keys_and_raw_values() -> None:
    raw_entity = _sequence(entity_id="alice")
    with pytest.raises(ValueError, match="synthetic/coarse entity identifier"):
        generate_representation_report([raw_entity])

    raw_sequence = _sequence(sequence_id="alice-session")
    with pytest.raises(ValueError, match="synthetic/coarse sequence identifier"):
        generate_representation_report([raw_sequence])

    raw_key = _sequence()
    raw_key["destination_ip"] = "198.51.100.10"
    with pytest.raises(ValueError, match="unsafe raw field"):
        generate_representation_report([raw_key])

    raw_value = _sequence()
    raw_value["service"] = "api.example.test"
    with pytest.raises(ValueError, match="unsafe raw identifier"):
        generate_representation_report([raw_value])

    evidence = generate_representation_report([_sequence()])["rows"][0]
    evidence["tokens"] = ["service:api.example.test"]
    with pytest.raises(ValueError, match="sanitized"):
        validate_representation_evidence_row(evidence)


def test_finite_bounded_risk_and_numeric_validation() -> None:
    bad_number = _sequence()
    bad_number["bytes_total"] = float("nan")
    with pytest.raises(ValueError, match="bytes_total must be a finite number"):
        generate_representation_report([bad_number])

    report = generate_representation_report(
        [
            _sequence("seq-001", "2026-01-01T00:00:00Z"),
            _sequence("seq-002", "2026-01-01T00:05:00Z"),
        ]
    )

    for row in report["rows"]:
        assert 0.0 <= row["embedding_novelty_score"] <= 1.0
        assert 0.0 <= row["representation_risk"] <= 1.0
        assert all(-1.0 <= value <= 1.0 for value in row["embedding"])


def test_stable_dump_and_non_strict_json_rejection(tmp_path: Path) -> None:
    report = generate_representation_report([_sequence()])
    output = tmp_path / "representation-report.json"
    dump_report(report, output)

    assert json.loads(output.read_text(encoding="utf-8")) == report
    assert output.read_text(encoding="utf-8").endswith("\n")

    with pytest.raises(ValueError, match="Out of range float values"):
        dump_report(
            {"schema_version": REPORT_SCHEMA_VERSION, "risk": float("nan")},
            tmp_path / "bad-report.json",
        )

    bad_input = tmp_path / "bad-input.jsonl"
    bad_input.write_text(
        '{"schema_version":"traffic_sequence_row.v0","sequence_id":"seq-bad",'
        '"entity_id":"asset-alpha","window_start":"2026-01-01T00:00:00Z",'
        '"bytes_total":NaN}\n',
        encoding="utf-8",
    )
    with pytest.raises(ValueError, match="non-strict JSON constant"):
        load_traffic_sequence_rows(bad_input)


def test_representation_evidence_converts_to_model_score_rows() -> None:
    report = generate_representation_report([_sequence()])
    score_rows = representation_evidence_to_score_rows(report)

    assert score_rows == [
        {
            "schema_version": "model_score_row.v0",
            "entity_id": "asset-alpha",
            "window_start": "2026-01-01T00:00:00Z",
            "scores": {
                MODEL_ID: {
                    "risk": 0.0,
                    "scale": "risk",
                    "family": MODEL_FAMILY,
                    "evidence": [
                        {
                            "sequence_id": "seq-001",
                            "tokens": report["rows"][0]["tokens"],
                            "token_count": 10,
                            "embedding_dimensions": 16,
                            "embedding_novelty_score": 0.0,
                            "rare_token_count": 10,
                            "representation_risk": 0.0,
                        }
                    ],
                }
            },
        }
    ]
