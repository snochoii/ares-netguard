from __future__ import annotations

import json
from pathlib import Path

import pytest

from ares_netguard.models.disagreement import dump_report, generate_disagreement_report


def _row(scores: dict[str, object]) -> dict[str, object]:
    return {
        "schema_version": "model_score_row.v0",
        "entity_id": "host-a",
        "window_start": "2026-01-01T00:00:00Z",
        "scores": scores,
    }


def test_normalizes_percentile_and_inverted_scores() -> None:
    report = generate_disagreement_report(
        [
            _row(
                {
                    "percentile_model": {"score": 87, "scale": "percentile"},
                    "low_is_risky_model": {"score": 0.2, "scale": "inverted_risk"},
                }
            )
        ]
    )

    scores = report["model_score_matrix"][0]["scores"]
    assert scores == {
        "low_is_risky_model": 0.8,
        "percentile_model": 0.87,
    }


def test_high_agreement_has_consensus_without_outlier() -> None:
    report = generate_disagreement_report(
        [
            _row(
                {
                    "isolation_forest": {"risk": 0.8},
                    "pyod_ecod": {"risk": 0.82},
                    "river_hst": {"risk": 0.79},
                }
            )
        ]
    )

    assert report["consensus_risk"] == 0.803333
    assert report["model_disagreement_score"] == 0.03
    assert report["model_agreement_score"] == 0.97
    assert report["row_reports"][0]["outlier_model"] is None


def test_disagreement_selects_dissenting_outlier() -> None:
    report = generate_disagreement_report(
        [
            _row(
                {
                    "isolation_forest": {"risk": 0.9},
                    "pyod_ecod": {"risk": 0.88},
                    "graph_novelty": {"risk": 0.1},
                }
            )
        ]
    )

    assert report["model_disagreement_score"] == 0.8
    assert report["row_reports"][0]["outlier_model"] == "graph_novelty"
    assert report["outlier_model"] == "graph_novelty"
    assert report["top_dissenting_models"][0] == {
        "model_id": "graph_novelty",
        "mean_deviation": 0.526667,
    }


def test_missing_model_score_does_not_break_row_summary() -> None:
    report = generate_disagreement_report([_row({"river_hst": {"risk": 0.42}})])

    assert report["model_agreement_score"] == 1.0
    assert report["model_disagreement_score"] == 0.0
    assert report["consensus_risk"] == 0.42
    assert report["outlier_model"] is None


def test_invalid_out_of_range_score_is_rejected() -> None:
    with pytest.raises(ValueError, match="outside 0.0..1.0"):
        generate_disagreement_report([_row({"isolation_forest": {"risk": 1.2}})])


def test_non_finite_score_is_rejected() -> None:
    with pytest.raises(ValueError, match="finite number"):
        generate_disagreement_report([_row({"isolation_forest": {"risk": float("nan")}})])


def test_boolean_score_is_rejected() -> None:
    with pytest.raises(ValueError, match="finite number"):
        generate_disagreement_report([_row({"isolation_forest": {"risk": True}})])


def test_row_schema_version_is_required() -> None:
    row = _row({"isolation_forest": {"risk": 0.5}})
    row["schema_version"] = "model_score_row.v1"

    with pytest.raises(ValueError, match="schema_version 'model_score_row.v0'"):
        generate_disagreement_report([row])


def test_row_must_be_object() -> None:
    with pytest.raises(ValueError, match="score row must be an object"):
        generate_disagreement_report([["not", "an", "object"]])  # type: ignore[list-item]


def test_dump_report_rejects_non_strict_json(tmp_path: Path) -> None:
    with pytest.raises(ValueError, match="Out of range float values"):
        dump_report({"schema_version": "bad", "risk": float("nan")}, tmp_path / "report.json")


def test_empty_evidence_and_stable_json_output() -> None:
    report = generate_disagreement_report([_row({"river_hst": {"risk": 0.68, "evidence": []}})])

    assert report["evidence_by_model"] == {}
    assert json.dumps(report, sort_keys=True) == json.dumps(report, sort_keys=True)
