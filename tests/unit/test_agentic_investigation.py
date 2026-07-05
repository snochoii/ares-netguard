from __future__ import annotations

import json
from pathlib import Path

import pytest

from ares_netguard.investigation.agentic_layer import (
    REPORT_SCHEMA_VERSION,
    dump_report,
    generate_investigation_report,
    load_report,
    validate_investigation_report,
)


def _disagreement_report() -> dict[str, object]:
    return {
        "schema_version": "model_disagreement_report.v0",
        "model_score_matrix": [],
        "model_agreement_score": 0.7,
        "model_disagreement_score": 0.3,
        "consensus_risk": 0.8,
        "outlier_model": None,
        "outlier_models": [],
        "top_supporting_models": [],
        "top_dissenting_models": [],
        "evidence_by_model": {},
        "row_reports": [
            {
                "schema_version": "model_score_row.v0",
                "entity_id": "host-alpha",
                "window_start": "2026-01-01T00:00:00Z",
                "scores": {
                    "isolation_forest": 0.91,
                    "pyod_ecod": 0.88,
                    "river_hst": 0.9,
                },
                "agreement_score": 0.97,
                "disagreement_score": 0.03,
                "consensus_risk": 0.896667,
                "outlier_model": None,
                "outlier_models": [],
                "evidence_by_model": {
                    "isolation_forest": ["external destination diversity spike"],
                    "pyod_ecod": ["rare feature vector percentile"],
                    "river_hst": ["online anomaly warmup complete"],
                },
            },
            {
                "schema_version": "model_score_row.v0",
                "entity_id": "host-beta",
                "window_start": "2026-01-01T00:05:00Z",
                "scores": {
                    "isolation_forest": 0.84,
                    "pyod_copod": 0.81,
                    "graph_novelty": 0.12,
                },
                "agreement_score": 0.28,
                "disagreement_score": 0.72,
                "consensus_risk": 0.59,
                "outlier_model": "graph_novelty",
                "outlier_models": ["graph_novelty"],
                "evidence_by_model": {
                    "isolation_forest": ["bytes outlier"],
                    "pyod_copod": ["tail probability breach"],
                    "graph_novelty": ["known service relationship"],
                },
            },
            {
                "schema_version": "model_score_row.v0",
                "entity_id": "host-gamma",
                "window_start": "2026-01-01T00:10:00Z",
                "scores": {
                    "suricata_alert": 0.7,
                    "river_hst": 0.68,
                },
                "agreement_score": 0.98,
                "disagreement_score": 0.02,
                "consensus_risk": 0.69,
                "outlier_model": None,
                "outlier_models": [],
                "evidence_by_model": {
                    "suricata_alert": ["synthetic alert severity high"],
                    "river_hst": [],
                },
            },
        ],
    }


def _representation_report() -> dict[str, object]:
    return {
        "schema_version": "traffic_representation_report.v0",
        "model_id": "self_supervised_representation",
        "model_family": "experimental_self_supervised",
        "embedding_dimensions": 4,
        "sequence_count": 1,
        "token_field_order": ["protocol"],
        "rows": [
            {
                "sequence_id": "seq-alpha",
                "entity_id": "host-alpha",
                "window_start": "2026-01-01T00:00:00Z",
                "tokens": ["protocol:tcp"],
                "token_count": 1,
                "embedding": [1.0, 0.0, 0.0, 0.0],
                "embedding_dimensions": 4,
                "embedding_novelty_score": 0.8,
                "rare_token_count": 1,
                "representation_risk": 0.8,
                "model_id": "self_supervised_representation",
                "model_family": "experimental_self_supervised",
            }
        ],
    }


def test_generates_deterministic_hypotheses_with_required_fields() -> None:
    report = generate_investigation_report(_disagreement_report())

    validate_investigation_report(report)
    assert report["schema_version"] == REPORT_SCHEMA_VERSION
    assert [row["hypothesis_id"] for row in report["rows"]] == [
        "hyp-0001",
        "hyp-0002",
        "hyp-0003",
    ]
    assert all(row["human_review_required"] is True for row in report["rows"])
    assert any("high consensus" in row["claim"] for row in report["rows"])
    assert any("outlier model" in row["claim"] for row in report["rows"])
    assert any("sparse or missing" in row["claim"] for row in report["rows"])


def test_optional_evidence_report_adds_reference_only_local_match() -> None:
    report = generate_investigation_report(
        _disagreement_report(),
        evidence_reports=[_representation_report()],
    )

    local_match = [
        row for row in report["rows"] if "matching local evidence reports" in row["claim"]
    ]
    assert len(local_match) == 1
    rendered = json.dumps(local_match, sort_keys=True)
    assert "traffic_representation_report.v0" in rendered
    assert "protocol:tcp" not in rendered
    assert "embedding" not in rendered


def test_output_rows_have_exact_hypothesis_contract() -> None:
    report = generate_investigation_report(_disagreement_report())

    expected_fields = {
        "hypothesis_id",
        "claim",
        "supporting_evidence_refs",
        "refuting_evidence_refs",
        "missing_evidence",
        "confidence",
        "recommended_next_query",
        "human_review_required",
    }
    assert all(set(row) == expected_fields for row in report["rows"])


def test_privacy_rejects_raw_identifiers() -> None:
    report = _disagreement_report()
    row = report["row_reports"][0]  # type: ignore[index]
    row["evidence_by_model"]["isolation_forest"] = ["connected to 192.168.1.10"]  # type: ignore[index]

    with pytest.raises(ValueError, match="unsafe raw identifier"):
        generate_investigation_report(report)


def test_payload_field_is_rejected() -> None:
    report = _disagreement_report()
    row = report["row_reports"][0]  # type: ignore[index]
    row["evidence_by_model"]["isolation_forest"] = [{"payload": "opaque"}]  # type: ignore[index]

    with pytest.raises(ValueError, match="forbidden raw field"):
        generate_investigation_report(report)


def test_non_finite_numbers_are_rejected() -> None:
    report = _disagreement_report()
    row = report["row_reports"][0]  # type: ignore[index]
    row["scores"]["isolation_forest"] = float("nan")  # type: ignore[index]

    with pytest.raises(ValueError, match="finite"):
        generate_investigation_report(report)


def test_strict_json_loader_rejects_nan(tmp_path: Path) -> None:
    path = tmp_path / "bad.json"
    path.write_text(
        '{"schema_version":"model_disagreement_report.v0","risk":NaN}', encoding="utf-8"
    )

    with pytest.raises(ValueError, match="non-strict JSON constant"):
        load_report(path)


def test_unknown_schema_is_rejected(tmp_path: Path) -> None:
    path = tmp_path / "bad.json"
    path.write_text('{"schema_version":"unknown_report.v0","rows":[]}', encoding="utf-8")

    with pytest.raises(ValueError, match="unknown report schema_version"):
        load_report(path)


def test_dump_report_rejects_non_strict_json(tmp_path: Path) -> None:
    report = generate_investigation_report(_disagreement_report())
    report["rows"][0]["confidence"] = float("inf")  # type: ignore[index]

    with pytest.raises(ValueError, match="finite"):
        dump_report(report, tmp_path / "report.json")
