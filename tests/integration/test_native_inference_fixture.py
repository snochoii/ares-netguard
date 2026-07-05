from __future__ import annotations

import json
from pathlib import Path

from ares_netguard.models.disagreement import generate_disagreement_report
from ares_netguard.native_inference.adapters import (
    dump_score_rows,
    load_feature_rows,
    load_manifest,
    score_feature_rows,
)


def test_fixture_scores_rows_and_feeds_disagreement_report(tmp_path: Path) -> None:
    manifest = Path("tests/fixtures/native_inference/manifest.json")
    feature_rows = Path("tests/fixtures/native_inference/feature_rows.jsonl")
    output = tmp_path / "native-inference-score-rows.json"

    score_rows = score_feature_rows(load_manifest(manifest), load_feature_rows(feature_rows))
    dump_score_rows(score_rows, output)
    persisted = json.loads(output.read_text(encoding="utf-8"))

    assert [row["schema_version"] for row in persisted] == ["model_score_row.v0"] * 3
    assert persisted[0]["scores"]["stdlib_linear_native"]["risk"] == 0.386986
    assert persisted[1]["scores"]["stdlib_linear_native"]["risk"] == 0.958513
    assert persisted[2]["scores"]["stdlib_linear_native"]["risk"] == 0.197816
    assert persisted == json.loads(output.read_text(encoding="utf-8"))

    disagreement_report = generate_disagreement_report(persisted)
    assert "stdlib_linear_native" in disagreement_report["evidence_by_model"]
    assert disagreement_report["model_score_matrix"][0]["scores"]["stdlib_linear_native"] >= 0.0
    assert disagreement_report["row_reports"][0]["schema_version"] == "model_score_row.v0"
