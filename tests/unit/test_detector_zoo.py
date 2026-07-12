from __future__ import annotations

import copy
import json
import math
from pathlib import Path
from typing import Any

import pytest

from ares_netguard.features import evidence_windows
from ares_netguard.models import detector_zoo

FIXTURE = Path("tests/fixtures/detector_zoo/synthetic_events.jsonl")
EXPECTED_MODEL_IDS = (
    "pyod_ecod",
    "pyod_copod",
    "pyod_hbos",
    "isolation_forest",
    "river_hst",
)
EXPECTED_SETTINGS = {
    "pyod_ecod": {"contamination": 0.1, "n_jobs": 1},
    "pyod_copod": {"contamination": 0.1, "n_jobs": 1},
    "pyod_hbos": {"contamination": 0.1, "n_bins": 10},
    "isolation_forest": {
        "contamination": 0.1,
        "n_estimators": 100,
        "random_state": 42,
        "n_jobs": 1,
    },
    "river_hst": {
        "n_trees": 10,
        "height": 6,
        "window_size": 16,
        "warmup_rows": 16,
        "seed": 42,
        "score_before_learn": True,
    },
}


@pytest.fixture(scope="module")
def feature_report() -> dict[str, Any]:
    return evidence_windows.generate_feature_window_report(
        evidence_windows.load_synthetic_telemetry_events(FIXTURE)
    )


@pytest.fixture(scope="module")
def score_rows(feature_report: dict[str, Any]) -> list[dict[str, Any]]:
    return detector_zoo.generate_detector_score_rows(feature_report)


def test_executes_fixed_detector_set_deterministically(
    feature_report: dict[str, Any],
    score_rows: list[dict[str, Any]],
) -> None:
    repeated = detector_zoo.generate_detector_score_rows(feature_report)

    assert repeated == score_rows
    assert len(score_rows) == 48
    assert [row["window_start"] for row in score_rows] == sorted(
        row["window_start"] for row in score_rows
    )
    assert detector_zoo.BATCH_MODEL_IDS == EXPECTED_MODEL_IDS[:4]
    assert detector_zoo.MODEL_IDS == EXPECTED_MODEL_IDS
    assert detector_zoo.MODEL_SETTINGS == EXPECTED_SETTINGS
    assert detector_zoo.PYOD_VERSION == "3.6.1"
    assert detector_zoo.RIVER_VERSION == "0.25.0"
    assert all(set(row["scores"]) == set(EXPECTED_MODEL_IDS[:4]) for row in score_rows[:16])
    assert all(set(row["scores"]) == set(EXPECTED_MODEL_IDS) for row in score_rows[16:])
    assert {model_id for row in score_rows for model_id in row["scores"]} == set(EXPECTED_MODEL_IDS)

    for row in score_rows:
        for model_id, entry in row["scores"].items():
            risk = entry["risk"]
            assert not isinstance(risk, bool)
            assert math.isfinite(risk)
            assert 0.0 <= risk <= 1.0
            assert risk == round(risk, 6)
            provenance = entry["evidence"][0]
            assert provenance["feature_columns"] == list(detector_zoo.FEATURE_COLUMNS)
            assert "window_size_minutes" not in provenance["feature_columns"]
            assert provenance["settings"] == EXPECTED_SETTINGS[model_id]
            assert provenance["synthetic_only"] is True
            assert provenance["local_only"] is True
            assert provenance["model_persisted"] is False
            assert provenance["deployment_allowed"] is False


def test_shuffled_report_still_emits_chronological_scores(
    feature_report: dict[str, Any],
    score_rows: list[dict[str, Any]],
) -> None:
    shuffled = copy.deepcopy(feature_report)
    shuffled["rows"] = list(reversed(shuffled["rows"]))

    assert detector_zoo.generate_detector_score_rows(shuffled) == score_rows


def test_late_anomaly_ranks_high_across_detector_families(
    score_rows: list[dict[str, Any]],
) -> None:
    anomaly = score_rows[-1]
    risks = {model_id: entry["risk"] for model_id, entry in anomaly["scores"].items()}

    assert anomaly["window_start"] == "2026-02-01T03:55:00Z"
    assert risks["isolation_forest"] == 1.0
    assert risks["river_hst"] == 1.0
    assert sum(risks[model_id] >= 0.95 for model_id in EXPECTED_MODEL_IDS) >= 4


def test_fixed_constructor_settings_match_emitted_provenance(
    score_rows: list[dict[str, Any]],
) -> None:
    batch_detectors = dict(detector_zoo._batch_detectors())
    for model_id in EXPECTED_MODEL_IDS[:4]:
        for setting, expected in EXPECTED_SETTINGS[model_id].items():
            assert getattr(batch_detectors[model_id], setting) == expected

    river_pipeline = detector_zoo._river_pipeline()
    assert list(river_pipeline.steps) == ["MinMaxScaler", "HalfSpaceTrees"]
    river_detector = river_pipeline.steps["HalfSpaceTrees"]
    for setting in ("n_trees", "height", "window_size", "seed"):
        assert getattr(river_detector, setting) == EXPECTED_SETTINGS["river_hst"][setting]

    latest_scores = score_rows[-1]["scores"]
    for model_id in EXPECTED_MODEL_IDS:
        provenance = latest_scores[model_id]["evidence"][0]
        assert provenance["settings"] == EXPECTED_SETTINGS[model_id]


def test_tie_aware_empirical_rank_formula_is_frozen() -> None:
    assert detector_zoo._tie_aware_empirical_ranks([1.0, 2.0, 2.0, 4.0]) == [
        0.0,
        0.5,
        0.5,
        1.0,
    ]
    assert detector_zoo._tie_aware_empirical_ranks([7.0]) == [0.5]


def test_river_scores_before_learning_and_omits_warmup(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    calls: list[tuple[str, int]] = []

    class FakePipeline:
        state = 0

        def score_one(self, features: dict[str, float]) -> float:
            assert "event_count" in features
            calls.append(("score", self.state))
            return float(self.state)

        def learn_one(self, features: dict[str, float]) -> None:
            assert "event_count" in features
            calls.append(("learn", self.state))
            self.state += 1

    monkeypatch.setattr(detector_zoo, "_river_pipeline", FakePipeline)
    features = [
        {column: float(index + 1) for column in detector_zoo.FEATURE_COLUMNS} for index in range(18)
    ]

    risks = detector_zoo._river_risks(features)

    assert risks[:16] == [None] * 16
    assert risks[16:] == [0.5, 1.0]
    assert calls == [call for index in range(18) for call in (("score", index), ("learn", index))]


def test_river_expanding_ranks_do_not_use_future_observations(
    feature_report: dict[str, Any],
) -> None:
    observations = detector_zoo._five_minute_observations(feature_report)
    features = [row["features"] for row in observations]

    prefix = detector_zoo._river_risks(features[:32])
    complete = detector_zoo._river_risks(features)

    assert prefix == complete[:32]


def test_rejects_schema_drift(feature_report: dict[str, Any]) -> None:
    tampered = copy.deepcopy(feature_report)
    tampered["schema_version"] = "telemetry_feature_window_report.v1"

    with pytest.raises(ValueError, match="schema_version"):
        detector_zoo.generate_detector_score_rows(tampered)


def test_rejects_missing_five_minute_rows(feature_report: dict[str, Any]) -> None:
    tampered = copy.deepcopy(feature_report)
    tampered["window_sizes_minutes"] = [1]
    tampered["rows"] = [
        row for row in tampered["rows"] if row["features"]["window_size_minutes"] == 1
    ]
    tampered["row_count"] = len(tampered["rows"])

    with pytest.raises(ValueError, match="five-minute observations"):
        detector_zoo.generate_detector_score_rows(tampered)


def test_rejects_fewer_than_32_five_minute_rows(feature_report: dict[str, Any]) -> None:
    tampered = copy.deepcopy(feature_report)
    one_minute = [row for row in tampered["rows"] if row["features"]["window_size_minutes"] == 1]
    five_minute = [row for row in tampered["rows"] if row["features"]["window_size_minutes"] == 5][
        :31
    ]
    tampered["rows"] = one_minute + five_minute
    tampered["row_count"] = len(tampered["rows"])

    with pytest.raises(ValueError, match="at least 32"):
        detector_zoo.generate_detector_score_rows(tampered)


def test_rejects_duplicate_five_minute_keys(feature_report: dict[str, Any]) -> None:
    tampered = copy.deepcopy(feature_report)
    duplicate = next(row for row in tampered["rows"] if row["features"]["window_size_minutes"] == 5)
    tampered["rows"].append(copy.deepcopy(duplicate))
    tampered["row_count"] += 1

    with pytest.raises(ValueError, match="duplicate feature window row"):
        detector_zoo.generate_detector_score_rows(tampered)


def test_rejects_non_finite_features(feature_report: dict[str, Any]) -> None:
    tampered = copy.deepcopy(feature_report)
    tampered["rows"][-1]["features"]["event_count"] = math.inf

    with pytest.raises(ValueError, match="finite number"):
        detector_zoo.generate_detector_score_rows(tampered)


def test_rejects_unsafe_identifiers(feature_report: dict[str, Any]) -> None:
    tampered = copy.deepcopy(feature_report)
    tampered["rows"][-1]["entity_id"] = "192.0.2.10"

    with pytest.raises(ValueError, match="unsafe raw identifier|synthetic/coarse"):
        detector_zoo.generate_detector_score_rows(tampered)


def test_rejects_feature_key_drift(feature_report: dict[str, Any]) -> None:
    tampered = copy.deepcopy(feature_report)
    del tampered["rows"][-1]["features"]["bytes_out_total"]

    with pytest.raises(ValueError, match="feature keys drifted"):
        detector_zoo.generate_detector_score_rows(tampered)


def test_loader_rejects_non_strict_json_and_unknown_fields(
    tmp_path: Path,
    feature_report: dict[str, Any],
) -> None:
    non_strict = tmp_path / "non-strict.json"
    non_strict.write_text('{"schema_version": NaN}', encoding="utf-8")
    with pytest.raises(ValueError, match="non-strict JSON constant"):
        detector_zoo.load_feature_window_report(non_strict)

    drifted = copy.deepcopy(feature_report)
    drifted["unexpected"] = True
    drifted_path = tmp_path / "drifted.json"
    drifted_path.write_text(json.dumps(drifted), encoding="utf-8")
    with pytest.raises(ValueError, match="unexpected"):
        detector_zoo.load_feature_window_report(drifted_path)


def test_dump_enforces_runtime_output_roots(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
    score_rows: list[dict[str, Any]],
) -> None:
    output = tmp_path / "detector-score-rows.json"
    detector_zoo.dump_score_rows(score_rows, output)
    assert json.loads(output.read_text(encoding="utf-8")) == score_rows

    with pytest.raises(ValueError, match="ignored runtime roots"):
        detector_zoo.dump_score_rows(score_rows, Path("detector-score-rows.json"))

    repo_root = Path(__file__).resolve().parents[2]
    monkeypatch.chdir(repo_root / "src")
    with pytest.raises(ValueError, match="ignored runtime roots"):
        detector_zoo.dump_score_rows(score_rows, Path("../detector-score-rows.json"))

    with pytest.raises(ValueError, match="file, not a directory"):
        detector_zoo.dump_score_rows(score_rows, tmp_path)


def test_cli_loads_feature_report_and_writes_score_rows(tmp_path: Path) -> None:
    feature_path = tmp_path / "feature-report.json"
    output = tmp_path / "detector-score-rows.json"
    evidence_windows.dump_feature_window_report(
        evidence_windows.generate_feature_window_report(
            evidence_windows.load_synthetic_telemetry_events(FIXTURE)
        ),
        feature_path,
    )

    assert detector_zoo.main([str(feature_path), str(output)]) == 0
    persisted = json.loads(output.read_text(encoding="utf-8"))
    assert len(persisted) == 48
    assert persisted[-1]["scores"]["river_hst"]["risk"] == 1.0
