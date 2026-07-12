"""Execute a bounded PyOD/River detector zoo over synthetic feature windows.

The v0 producer fits four batch detectors and one online detector entirely in
memory. It accepts only the strict, local, synthetic telemetry feature report
contract. It does not persist fitted models, inspect raw telemetry, download
model weights, perform capture, call external services, or enable deployment.
"""

from __future__ import annotations

import argparse
import json
import math
from collections.abc import Mapping, Sequence
from datetime import UTC, datetime
from importlib import metadata
from pathlib import Path
from typing import Any

from ares_netguard.features import evidence_windows
from ares_netguard.models import score_row_composer
from ares_netguard.models.disagreement import ROW_SCHEMA_VERSION

JsonMap = dict[str, Any]

MIN_OBSERVATIONS = 32
RIVER_WARMUP_ROWS = 16
PYOD_VERSION = "3.6.1"
RIVER_VERSION = "0.25.0"

BATCH_MODEL_IDS = (
    "pyod_ecod",
    "pyod_copod",
    "pyod_hbos",
    "isolation_forest",
)
MODEL_IDS = (*BATCH_MODEL_IDS, "river_hst")
FEATURE_COLUMNS = tuple(
    name for name in evidence_windows.FEATURE_FIELDS if name != "window_size_minutes"
)

MODEL_FAMILIES = {
    "pyod_ecod": "pyod",
    "pyod_copod": "pyod",
    "pyod_hbos": "pyod",
    "isolation_forest": "baseline",
    "river_hst": "river",
}
MODEL_SETTINGS: dict[str, JsonMap] = {
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
        "window_size": RIVER_WARMUP_ROWS,
        "warmup_rows": RIVER_WARMUP_ROWS,
        "seed": 42,
        "score_before_learn": True,
    },
}


def load_feature_window_report(path: str | Path) -> JsonMap:
    """Load and strictly validate one telemetry_feature_window_report.v0 file."""
    source = Path(path)
    if source.is_dir():
        raise ValueError(f"feature report path must be a file, not a directory: {source}")
    payload = _loads_strict(source.read_text(encoding="utf-8"))
    if not isinstance(payload, Mapping):
        raise ValueError("feature report payload must be an object")
    evidence_windows.validate_feature_window_report(payload)
    return _clone_json(payload)


def generate_detector_score_rows(report: Mapping[str, Any]) -> list[JsonMap]:
    """Fit the bounded detector cohort and emit strict model_score_row.v0 rows."""
    observations = _five_minute_observations(report)
    feature_vectors = [observation["features"] for observation in observations]

    batch_risks = _batch_risks(feature_vectors)
    river_risks = _river_risks(feature_vectors)

    rows: list[JsonMap] = []
    for index, observation in enumerate(observations):
        scores = {
            model_id: _score_entry(
                model_id,
                batch_risks[model_id][index],
                normalization="cohort_tie_aware_empirical_rank",
            )
            for model_id in BATCH_MODEL_IDS
        }
        river_risk = river_risks[index]
        if river_risk is not None:
            scores["river_hst"] = _score_entry(
                "river_hst",
                river_risk,
                normalization="expanding_tie_aware_empirical_rank",
            )

        row = {
            "schema_version": ROW_SCHEMA_VERSION,
            "entity_id": observation["entity_id"],
            "window_start": observation["window_start"],
            "scores": scores,
        }
        score_row_composer.validate_score_row(row)
        rows.append(row)
    return rows


def dump_score_rows(rows: Sequence[Mapping[str, Any]], path: str | Path) -> None:
    """Write strict score rows only to external or ignored repository roots."""
    output = _validated_output_path(path)
    rows_to_write = [_clone_json(row) for row in rows]
    for index, row in enumerate(rows_to_write):
        score_row_composer.validate_score_row(row, label=f"score row {index}")
    output.write_text(
        json.dumps(rows_to_write, allow_nan=False, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def _five_minute_observations(report: Mapping[str, Any]) -> list[JsonMap]:
    evidence_windows.validate_feature_window_report(report)
    selected: list[JsonMap] = []
    seen: set[tuple[str, str]] = set()

    for raw_row in report["rows"]:
        if raw_row["features"]["window_size_minutes"] != 5:
            continue
        key = (str(raw_row["entity_id"]), str(raw_row["window_start"]))
        if key in seen:
            raise ValueError("duplicate five-minute entity/window observation")
        seen.add(key)
        selected.append(
            {
                "entity_id": key[0],
                "window_start": key[1],
                "features": {
                    column: _finite_feature(raw_row["features"][column], column)
                    for column in FEATURE_COLUMNS
                },
            }
        )

    if not selected:
        raise ValueError("feature report requires five-minute observations")
    if len(selected) < MIN_OBSERVATIONS:
        raise ValueError(
            f"detector zoo requires at least {MIN_OBSERVATIONS} unique five-minute observations"
        )

    return sorted(
        selected,
        key=lambda row: (
            _timestamp_sort_key(row["window_start"]),
            row["entity_id"],
            row["window_start"],
        ),
    )


def _batch_risks(feature_vectors: Sequence[Mapping[str, float]]) -> dict[str, list[float]]:
    matrix = [[features[column] for column in FEATURE_COLUMNS] for features in feature_vectors]
    risks: dict[str, list[float]] = {}
    for model_id, detector in _batch_detectors():
        detector.fit(matrix)
        raw_scores = [_finite_score(value, model_id) for value in detector.decision_scores_]
        if len(raw_scores) != len(feature_vectors):
            raise ValueError(f"{model_id} returned an unexpected score count")
        risks[model_id] = _tie_aware_empirical_ranks(raw_scores)
    return risks


def _batch_detectors() -> list[tuple[str, Any]]:
    _require_library_version("pyod", PYOD_VERSION)
    from pyod.models.copod import COPOD
    from pyod.models.ecod import ECOD
    from pyod.models.hbos import HBOS
    from pyod.models.iforest import IForest

    return [
        ("pyod_ecod", ECOD(**MODEL_SETTINGS["pyod_ecod"])),
        ("pyod_copod", COPOD(**MODEL_SETTINGS["pyod_copod"])),
        ("pyod_hbos", HBOS(**MODEL_SETTINGS["pyod_hbos"])),
        ("isolation_forest", IForest(**MODEL_SETTINGS["isolation_forest"])),
    ]


def _river_risks(feature_vectors: Sequence[Mapping[str, float]]) -> list[float | None]:
    detector = _river_pipeline()
    raw_history: list[float] = []
    risks: list[float | None] = []

    for index, raw_features in enumerate(feature_vectors):
        features = dict(raw_features)
        raw_score = _finite_score(detector.score_one(features), "river_hst")
        if index < RIVER_WARMUP_ROWS:
            risks.append(None)
        else:
            raw_history.append(raw_score)
            risks.append(_tie_aware_empirical_ranks(raw_history)[-1])
        detector.learn_one(features)
    return risks


def _river_pipeline() -> Any:
    _require_library_version("river", RIVER_VERSION)
    from river import anomaly, preprocessing

    settings = MODEL_SETTINGS["river_hst"]
    return preprocessing.MinMaxScaler() | anomaly.HalfSpaceTrees(
        n_trees=settings["n_trees"],
        height=settings["height"],
        window_size=settings["window_size"],
        seed=settings["seed"],
    )


def _tie_aware_empirical_ranks(raw_values: Sequence[float]) -> list[float]:
    """Return average-tie empirical ranks on 0..1, preserving input order."""
    values = [_finite_score(value, "empirical_rank") for value in raw_values]
    if not values:
        return []
    if len(values) == 1:
        return [0.5]

    ordered = sorted(enumerate(values), key=lambda item: (item[1], item[0]))
    ranks = [0.0] * len(values)
    start = 0
    while start < len(ordered):
        end = start + 1
        while end < len(ordered) and ordered[end][1] == ordered[start][1]:
            end += 1
        average_position = (start + end - 1) / 2
        rank = _round(average_position / (len(values) - 1))
        for position in range(start, end):
            ranks[ordered[position][0]] = rank
        start = end
    return ranks


def _score_entry(model_id: str, risk: float, *, normalization: str) -> JsonMap:
    library = "river" if model_id == "river_hst" else "pyod"
    version = RIVER_VERSION if library == "river" else PYOD_VERSION
    return {
        "risk": _bounded_risk(risk, model_id),
        "scale": "risk",
        "family": MODEL_FAMILIES[model_id],
        "evidence": [
            {
                "producer": "detector_zoo_v0",
                "execution_mode": "in_memory_library_model",
                "library": library,
                "library_version": version,
                "settings": dict(MODEL_SETTINGS[model_id]),
                "input_schema_version": evidence_windows.REPORT_SCHEMA_VERSION,
                "feature_schema_version": evidence_windows.FEATURE_ROW_SCHEMA_VERSION,
                "feature_columns": list(FEATURE_COLUMNS),
                "normalization": normalization,
                "cohort_scope": "global_synthetic_fixture",
                "local_only": True,
                "synthetic_only": True,
                "model_persisted": False,
                "deployment_allowed": False,
            }
        ],
    }


def _validated_output_path(path: str | Path) -> Path:
    output = Path(path)
    if output.is_dir():
        raise ValueError(f"output path must be a file, not a directory: {output}")
    resolved = output.resolve(strict=False)
    repo_root = _repository_root()
    if _is_relative_to(resolved, repo_root):
        allowed_roots = (
            repo_root / "data" / "features",
            repo_root / "data" / "models",
            repo_root / "data" / "reports",
            repo_root / ".runtime",
            repo_root / "artifacts",
        )
        if not any(_is_relative_to(resolved, root.resolve(strict=False)) for root in allowed_roots):
            raise ValueError("repository output paths must be under ignored runtime roots")
    return output


def _repository_root() -> Path:
    for parent in Path(__file__).resolve().parents:
        if (parent / "AGENTS.md").is_file() and (parent / "pyproject.toml").is_file():
            return parent.resolve()
    return Path.cwd().resolve()


def _is_relative_to(child: Path, parent: Path) -> bool:
    try:
        child.relative_to(parent)
    except ValueError:
        return False
    return True


def _timestamp_sort_key(value: str) -> datetime:
    return datetime.fromisoformat(value.replace("Z", "+00:00")).astimezone(UTC)


def _finite_feature(raw_value: Any, feature_name: str) -> float:
    if isinstance(raw_value, bool) or not isinstance(raw_value, int | float):
        raise ValueError(f"{feature_name} must be a finite numeric feature")
    value = float(raw_value)
    if not math.isfinite(value):
        raise ValueError(f"{feature_name} must be a finite numeric feature")
    return value


def _finite_score(raw_value: Any, model_id: str) -> float:
    if isinstance(raw_value, bool) or not isinstance(raw_value, int | float):
        raise ValueError(f"{model_id} returned a non-finite score")
    value = float(raw_value)
    if not math.isfinite(value):
        raise ValueError(f"{model_id} returned a non-finite score")
    return value


def _bounded_risk(raw_value: Any, model_id: str) -> float:
    value = _finite_score(raw_value, model_id)
    if value < 0.0 or value > 1.0:
        raise ValueError(f"{model_id} risk must be between 0 and 1")
    return _round(value)


def _require_library_version(distribution: str, expected: str) -> None:
    actual = metadata.version(distribution)
    if actual != expected:
        raise RuntimeError(
            f"detector zoo requires {distribution}=={expected}; installed version is {actual}"
        )


def _loads_strict(text: str) -> Any:
    def reject_constant(value: str) -> None:
        raise ValueError(f"non-strict JSON constant '{value}' is not allowed")

    return json.loads(text, parse_constant=reject_constant)


def _clone_json(value: Any) -> Any:
    return _loads_strict(json.dumps(value, allow_nan=False, sort_keys=True))


def _round(value: float) -> float:
    rounded = round(value, 6)
    return 0.0 if rounded == 0 else rounded


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Execute the synthetic in-memory detector zoo and emit score rows."
    )
    parser.add_argument("input", help="telemetry_feature_window_report.v0 JSON input")
    parser.add_argument("output", help="Path to write model_score_row.v0 JSON rows")
    args = parser.parse_args(argv)

    report = load_feature_window_report(args.input)
    dump_score_rows(generate_detector_score_rows(report), args.output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
