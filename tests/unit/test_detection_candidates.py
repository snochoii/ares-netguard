from __future__ import annotations

import json
from pathlib import Path

import pytest

from ares_netguard.detection_engineering.candidates import (
    CANDIDATE_LANGUAGES,
    DRAFT_MARKER,
    REPORT_SCHEMA_VERSION,
    dump_report,
    generate_candidate_report,
    load_report,
    validate_candidate_report,
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
        ],
    }


def test_generates_deterministic_candidates_for_eligible_patterns() -> None:
    report = generate_candidate_report(_disagreement_report())

    validate_candidate_report(report)
    assert report["schema_version"] == REPORT_SCHEMA_VERSION
    assert report["source_report_schema"] == "model_disagreement_report.v0"
    assert report["validation_summary"]["source_rows_considered"] == 2
    assert report["validation_summary"]["eligible_patterns"] == 2
    assert report["validation_summary"]["candidates_generated"] == 8
    assert [row["candidate_id"] for row in report["rows"]] == [
        "cand-v0-0000-high_consensus_risk-zeek",
        "cand-v0-0000-high_consensus_risk-sigma_like",
        "cand-v0-0000-high_consensus_risk-suricata_local",
        "cand-v0-0000-high_consensus_risk-siem_query",
        "cand-v0-0001-high_model_disagreement-zeek",
        "cand-v0-0001-high_model_disagreement-sigma_like",
        "cand-v0-0001-high_model_disagreement-suricata_local",
        "cand-v0-0001-high_model_disagreement-siem_query",
    ]


def test_candidate_rows_have_exact_contract_and_required_review_flags() -> None:
    report = generate_candidate_report(_disagreement_report())
    expected_fields = {
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

    assert all(set(row) == expected_fields for row in report["rows"])
    assert all(row["human_review_required"] is True for row in report["rows"])
    assert all(row["deployment_allowed"] is False for row in report["rows"])
    assert all(DRAFT_MARKER in row["draft"] for row in report["rows"])


def test_four_candidate_languages_are_emitted_per_pattern() -> None:
    report = generate_candidate_report(_disagreement_report())
    languages_by_kind = {
        kind: [row["candidate_language"] for row in report["rows"] if row["candidate_kind"] == kind]
        for kind in {"high_consensus_risk", "high_model_disagreement"}
    }

    assert languages_by_kind["high_consensus_risk"] == list(CANDIDATE_LANGUAGES)
    assert languages_by_kind["high_model_disagreement"] == list(CANDIDATE_LANGUAGES)


def test_source_refs_do_not_copy_evidence_blobs() -> None:
    report = generate_candidate_report(_disagreement_report())
    rendered = json.dumps(report, sort_keys=True)

    assert "external destination diversity spike" not in rendered
    assert "tail probability breach" not in rendered
    assert "known service relationship" not in rendered
    assert all(row["source_evidence_refs"] for row in report["rows"])
    assert all(
        ref["field_path"].startswith("[row_reports]")
        for row in report["rows"]
        for ref in row["source_evidence_refs"]
    )


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


def test_directory_input_is_rejected(tmp_path: Path) -> None:
    with pytest.raises(ValueError, match="not a directory"):
        load_report(tmp_path)


def test_privacy_rejects_raw_identifiers() -> None:
    report = _disagreement_report()
    row = report["row_reports"][0]  # type: ignore[index]
    row["evidence_by_model"]["isolation_forest"] = ["connected to 192.168.1.10"]  # type: ignore[index]

    with pytest.raises(ValueError, match="unsafe raw identifier"):
        generate_candidate_report(report)


@pytest.mark.parametrize("raw_key", ["192.168.1.10", "example.com", "payload"])
def test_privacy_rejects_raw_identifier_keys(raw_key: str) -> None:
    report = _disagreement_report()
    row = report["row_reports"][0]  # type: ignore[index]
    row["evidence_by_model"][raw_key] = []  # type: ignore[index]

    with pytest.raises(ValueError, match="unsafe raw identifier|forbidden raw field"):
        generate_candidate_report(report)


def test_oversized_values_are_rejected() -> None:
    report = _disagreement_report()
    row = report["row_reports"][0]  # type: ignore[index]
    row["evidence_by_model"]["isolation_forest"] = ["x" * 1025]  # type: ignore[index]

    with pytest.raises(ValueError, match="exceeds maximum string length"):
        generate_candidate_report(report)


def test_non_finite_numbers_are_rejected() -> None:
    report = _disagreement_report()
    row = report["row_reports"][0]  # type: ignore[index]
    row["scores"]["isolation_forest"] = float("inf")  # type: ignore[index]

    with pytest.raises(ValueError, match="finite"):
        generate_candidate_report(report)


def test_dump_report_rejects_non_strict_json(tmp_path: Path) -> None:
    report = generate_candidate_report(_disagreement_report())
    report["validation_summary"]["source_rows_considered"] = float("nan")  # type: ignore[index]

    with pytest.raises(ValueError, match="finite"):
        dump_report(report, tmp_path / "report.json")
