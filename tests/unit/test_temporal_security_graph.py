from __future__ import annotations

import json
from pathlib import Path

import pytest

from ares_netguard.graph.temporal_security_graph import (
    MODEL_FAMILY,
    MODEL_ID,
    REPORT_SCHEMA_VERSION,
    dump_report,
    generate_temporal_security_graph_report,
    load_temporal_graph_edge_rows,
    temporal_graph_evidence_to_score_rows,
    validate_temporal_graph_evidence_row,
)


def _edge(
    event_id: str,
    window_start: str,
    *,
    entity_id: str = "asset-alpha",
    source_node_type: str = "host",
    source_node_id: str = "host-alpha",
    edge_type: str = "connected_to",
    target_node_type: str = "service",
    target_node_id: str = "service-web",
    observed_count: object = 1,
) -> dict[str, object]:
    return {
        "schema_version": "temporal_graph_edge_row.v0",
        "event_id": event_id,
        "entity_id": entity_id,
        "window_start": window_start,
        "source_node_type": source_node_type,
        "source_node_id": source_node_id,
        "edge_type": edge_type,
        "target_node_type": target_node_type,
        "target_node_id": target_node_id,
        "observed_count": observed_count,
    }


def test_graph_feature_math_and_stable_output_order() -> None:
    rows = [
        _edge("edge-alpha-004", "2026-01-01T00:10:00Z", target_node_id="service-backup"),
        _edge(
            "edge-beta-002",
            "2026-01-01T00:05:00Z",
            entity_id="asset-beta",
            source_node_id="host-beta",
        ),
        _edge("edge-alpha-001", "2026-01-01T00:00:00Z"),
        _edge(
            "edge-beta-001",
            "2026-01-01T00:00:00Z",
            entity_id="asset-beta",
            source_node_id="host-beta",
        ),
        _edge("edge-alpha-003", "2026-01-01T00:10:00Z"),
        _edge("edge-alpha-002", "2026-01-01T00:05:00Z"),
        _edge(
            "edge-beta-003",
            "2026-01-01T00:10:00Z",
            entity_id="asset-beta",
            source_node_id="host-beta",
        ),
    ]

    report = generate_temporal_security_graph_report(rows)
    repeat_report = generate_temporal_security_graph_report(list(reversed(rows)))
    by_event = {row["event_id"]: row for row in report["rows"]}

    assert report == repeat_report
    assert report["schema_version"] == REPORT_SCHEMA_VERSION
    assert report["model_id"] == MODEL_ID
    assert report["model_family"] == MODEL_FAMILY
    assert [row["event_id"] for row in report["rows"]] == [
        "edge-alpha-001",
        "edge-alpha-002",
        "edge-alpha-003",
        "edge-alpha-004",
        "edge-beta-001",
        "edge-beta-002",
        "edge-beta-003",
    ]
    assert by_event["edge-alpha-001"]["warmup"] is True
    assert by_event["edge-alpha-001"]["graph_novelty_risk"] == 0.0
    assert by_event["edge-alpha-004"]["warmup"] is False
    assert by_event["edge-alpha-004"]["rare_edge_score"] == 1.0
    assert by_event["edge-alpha-004"]["new_neighbor_ratio"] == 0.5
    assert by_event["edge-alpha-004"]["degree_change_score"] == 0.5
    assert by_event["edge-alpha-004"]["graph_novelty_risk"] == 1.0
    assert by_event["edge-alpha-003"]["graph_novelty_risk"] == 0.5
    assert by_event["edge-beta-003"]["graph_novelty_risk"] == 0.0


def test_symmetric_edges_share_canonical_edge_key() -> None:
    rows = [
        _edge(
            "edge-sym-001",
            "2026-01-01T00:00:00Z",
            source_node_type="process",
            source_node_id="process-shell",
            edge_type="co_occurred",
            target_node_type="alert",
            target_node_id="alert-medium",
        ),
        _edge(
            "edge-sym-002",
            "2026-01-01T00:05:00Z",
            source_node_type="alert",
            source_node_id="alert-medium",
            edge_type="co_occurred",
            target_node_type="process",
            target_node_id="process-shell",
        ),
    ]

    report = generate_temporal_security_graph_report(
        rows,
        history_window=2,
        min_history_windows=1,
    )

    assert report["rows"][0]["canonical_edge_key"] == report["rows"][1]["canonical_edge_key"]
    assert report["rows"][1]["rare_edge_score"] == 0.0


def test_schema_validation_rejects_bad_rows() -> None:
    with pytest.raises(ValueError, match="temporal graph edge row must be an object"):
        generate_temporal_security_graph_report([["not", "an", "object"]])  # type: ignore[list-item]

    missing = _edge("edge-missing", "2026-01-01T00:00:00Z")
    del missing["target_node_id"]
    with pytest.raises(ValueError, match="missing required fields"):
        generate_temporal_security_graph_report([missing])

    with pytest.raises(ValueError, match="duplicate event_id"):
        generate_temporal_security_graph_report(
            [
                _edge("edge-dup", "2026-01-01T00:00:00Z"),
                _edge("edge-dup", "2026-01-01T00:05:00Z"),
            ]
        )

    bad_timestamp = _edge("edge-bad-time", "2026-01-01 00:00:00")
    with pytest.raises(ValueError, match="timezone"):
        generate_temporal_security_graph_report([bad_timestamp])

    bad_node_type = _edge("edge-bad-node", "2026-01-01T00:00:00Z", source_node_type="raw_host")
    with pytest.raises(ValueError, match="unknown node type"):
        generate_temporal_security_graph_report([bad_node_type])

    bad_edge_type = _edge("edge-bad-edge", "2026-01-01T00:00:00Z", edge_type="queried")
    with pytest.raises(ValueError, match="unknown edge type"):
        generate_temporal_security_graph_report([bad_edge_type])


def test_privacy_guard_rejects_raw_fields_and_values() -> None:
    raw_entity = _edge("edge-raw-entity", "2026-01-01T00:00:00Z", entity_id="alice")
    with pytest.raises(ValueError, match="synthetic/coarse entity identifier"):
        generate_temporal_security_graph_report([raw_entity])

    raw_key = _edge("edge-raw-key", "2026-01-01T00:00:00Z")
    raw_key["destination_ip"] = "198.51.100.10"
    with pytest.raises(ValueError, match="unsafe raw field"):
        generate_temporal_security_graph_report([raw_key])

    raw_ip = _edge("edge-raw-ip", "2026-01-01T00:00:00Z", target_node_id="192.0.2.10")
    with pytest.raises(ValueError, match="unsafe raw identifier"):
        generate_temporal_security_graph_report([raw_ip])

    raw_domain = _edge(
        "edge-raw-domain",
        "2026-01-01T00:00:00Z",
        target_node_id="api.example.test",
    )
    with pytest.raises(ValueError, match="unsafe raw identifier"):
        generate_temporal_security_graph_report([raw_domain])

    raw_email = _edge("edge-raw-email", "2026-01-01T00:00:00Z", source_node_id="alice@example.test")
    with pytest.raises(ValueError, match="unsafe raw identifier"):
        generate_temporal_security_graph_report([raw_email])

    raw_path = _edge("edge-raw-path", "2026-01-01T00:00:00Z", target_node_id="/home/alice/file")
    with pytest.raises(ValueError, match="unsafe raw identifier"):
        generate_temporal_security_graph_report([raw_path])

    evidence = generate_temporal_security_graph_report(
        [_edge("edge-safe-001", "2026-01-01T00:00:00Z")]
    )["rows"][0]
    evidence["canonical_edge_key"] = "host:host-alpha|connected_to|service:api.example.test"
    with pytest.raises(ValueError, match="unsafe raw identifier"):
        validate_temporal_graph_evidence_row(evidence)


def test_finite_bounded_risk_values_and_strict_json(tmp_path: Path) -> None:
    bad_count = _edge("edge-bad-count", "2026-01-01T00:00:00Z", observed_count=float("nan"))
    with pytest.raises(ValueError, match="observed_count must be a non-negative integer"):
        generate_temporal_security_graph_report([bad_count])

    report = generate_temporal_security_graph_report(
        [
            _edge("edge-alpha-001", "2026-01-01T00:00:00Z"),
            _edge("edge-alpha-002", "2026-01-01T00:05:00Z"),
            _edge("edge-alpha-003", "2026-01-01T00:10:00Z", target_node_id="service-backup"),
        ]
    )
    for row in report["rows"]:
        assert 0.0 <= row["rare_edge_score"] <= 1.0
        assert 0.0 <= row["new_neighbor_ratio"] <= 1.0
        assert 0.0 <= row["degree_change_score"] <= 1.0
        assert 0.0 <= row["graph_novelty_risk"] <= 1.0

    output = tmp_path / "graph-report.json"
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
        '{"schema_version":"temporal_graph_edge_row.v0","event_id":"edge-bad-json",'
        '"entity_id":"asset-alpha","window_start":"2026-01-01T00:00:00Z",'
        '"source_node_type":"host","source_node_id":"host-alpha",'
        '"edge_type":"connected_to","target_node_type":"service",'
        '"target_node_id":"service-web","observed_count":NaN}\n',
        encoding="utf-8",
    )
    with pytest.raises(ValueError, match="non-strict JSON constant"):
        load_temporal_graph_edge_rows(bad_input)


def test_graph_evidence_converts_to_model_score_rows() -> None:
    report = generate_temporal_security_graph_report(
        [
            _edge("edge-alpha-001", "2026-01-01T00:00:00Z"),
            _edge("edge-alpha-002", "2026-01-01T00:05:00Z"),
            _edge("edge-alpha-003", "2026-01-01T00:10:00Z"),
            _edge("edge-alpha-004", "2026-01-01T00:10:00Z", target_node_id="service-backup"),
        ]
    )
    score_rows = temporal_graph_evidence_to_score_rows(report)

    assert score_rows[-1] == {
        "schema_version": "model_score_row.v0",
        "entity_id": "asset-alpha",
        "window_start": "2026-01-01T00:10:00Z",
        "scores": {
            MODEL_ID: {
                "risk": 1.0,
                "scale": "risk",
                "family": MODEL_FAMILY,
                "evidence": [
                    {
                        "event_id": "edge-alpha-003",
                        "canonical_edge_key": "host:host-alpha|connected_to|service:service-web",
                        "source_node_type": "host",
                        "source_node_id": "host-alpha",
                        "edge_type": "connected_to",
                        "target_node_type": "service",
                        "target_node_id": "service-web",
                        "observed_count": 1,
                        "history_window_count": 2,
                        "warmup": False,
                        "rare_edge_score": 0.0,
                        "new_neighbor_ratio": 0.5,
                        "degree_change_score": 0.5,
                        "graph_novelty_risk": 0.5,
                    },
                    {
                        "event_id": "edge-alpha-004",
                        "canonical_edge_key": "host:host-alpha|connected_to|service:service-backup",
                        "source_node_type": "host",
                        "source_node_id": "host-alpha",
                        "edge_type": "connected_to",
                        "target_node_type": "service",
                        "target_node_id": "service-backup",
                        "observed_count": 1,
                        "history_window_count": 2,
                        "warmup": False,
                        "rare_edge_score": 1.0,
                        "new_neighbor_ratio": 0.5,
                        "degree_change_score": 0.5,
                        "graph_novelty_risk": 1.0,
                    },
                ],
            }
        },
    }
