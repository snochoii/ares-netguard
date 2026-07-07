from __future__ import annotations

import json
from pathlib import Path

import pytest

from ares_netguard.models import score_row_composer


def _row(
    entity_id: str,
    window_start: str,
    model_id: str,
    risk: float = 0.5,
) -> dict[str, object]:
    return {
        "schema_version": "model_score_row.v0",
        "entity_id": entity_id,
        "window_start": window_start,
        "scores": {
            model_id: {
                "risk": risk,
                "scale": "risk",
                "family": "synthetic",
                "evidence": [f"{model_id} evidence bucket"],
            }
        },
    }


def test_compose_score_rows_is_sorted_and_merges_sparse_windows() -> None:
    rows = score_row_composer.compose_score_rows(
        score_row_sources=[
            [
                _row("host-beta", "2026-01-01T00:05:00Z", "river_hst", 0.2),
                _row("host-alpha", "2026-01-01T00:00:00Z", "pyod_ecod", 0.8),
            ],
            [_row("host-alpha", "2026-01-01T00:00:00Z", "isolation_forest", 0.7)],
        ]
    )

    assert [(row["entity_id"], row["window_start"]) for row in rows] == [
        ("host-alpha", "2026-01-01T00:00:00Z"),
        ("host-beta", "2026-01-01T00:05:00Z"),
    ]
    assert list(rows[0]["scores"]) == ["isolation_forest", "pyod_ecod"]
    assert list(rows[1]["scores"]) == ["river_hst"]


def test_duplicate_entity_window_model_is_rejected() -> None:
    with pytest.raises(ValueError, match="duplicate score tuple"):
        score_row_composer.compose_score_rows(
            score_row_sources=[
                [_row("host-alpha", "2026-01-01T00:00:00Z", "pyod_ecod", 0.8)],
                [_row("host-alpha", "2026-01-01T00:00:00Z", "pyod_ecod", 0.9)],
            ]
        )


def test_invalid_score_row_schema_is_rejected() -> None:
    row = _row("host-alpha", "2026-01-01T00:00:00Z", "pyod_ecod")
    row["schema_version"] = "model_score_row.v1"

    with pytest.raises(ValueError, match="schema_version 'model_score_row.v0'"):
        score_row_composer.compose_score_rows(score_rows=[row])


def test_invalid_entity_id_is_rejected() -> None:
    with pytest.raises(ValueError, match="unsafe raw identifier"):
        score_row_composer.compose_score_rows(
            score_rows=[_row("192.168.1.20", "2026-01-01T00:00:00Z", "pyod_ecod")]
        )


def test_invalid_window_start_is_rejected() -> None:
    with pytest.raises(ValueError, match="ISO-8601 timestamp"):
        score_row_composer.compose_score_rows(score_rows=[_row("host-alpha", "bad", "pyod_ecod")])


def test_invalid_model_id_is_rejected() -> None:
    with pytest.raises(ValueError, match="sanitized model identifier"):
        score_row_composer.compose_score_rows(
            score_rows=[_row("host-alpha", "2026-01-01T00:00:00Z", "bad model")]
        )


def test_programmatic_non_finite_score_is_rejected() -> None:
    with pytest.raises(ValueError, match="finite numbers"):
        score_row_composer.compose_score_rows(
            score_rows=[_row("host-alpha", "2026-01-01T00:00:00Z", "pyod_ecod", float("inf"))]
        )


def test_unsafe_private_or_artifact_text_is_rejected() -> None:
    row = _row("host-alpha", "2026-01-01T00:00:00Z", "pyod_ecod")
    row["scores"]["pyod_ecod"]["evidence"] = ["model.onnx"]  # type: ignore[index]

    with pytest.raises(ValueError, match="unsafe raw identifier"):
        score_row_composer.compose_score_rows(score_rows=[row])


def test_non_strict_json_constants_are_rejected(tmp_path: Path) -> None:
    source = tmp_path / "rows.json"
    source.write_text(
        (
            '{"schema_version":"model_score_row.v0",'
            '"entity_id":"host-alpha",'
            '"window_start":"2026-01-01T00:00:00Z",'
            '"scores":{"pyod_ecod":{"risk":NaN}}}'
        ),
        encoding="utf-8",
    )

    with pytest.raises(ValueError, match="non-strict JSON constant"):
        score_row_composer.load_score_rows(source)


def test_invalid_report_schema_is_rejected(tmp_path: Path) -> None:
    source = tmp_path / "residual-report.json"
    source.write_text(
        json.dumps({"schema_version": "time_series_residual_report.v1", "rows": []}),
        encoding="utf-8",
    )

    with pytest.raises(ValueError, match="time_series_residual_report.v0"):
        score_row_composer.load_residual_report(source)


def test_dump_score_rows_writes_bare_strict_json_list(tmp_path: Path) -> None:
    output = tmp_path / "composed-model-score-rows.json"
    rows = score_row_composer.compose_score_rows(
        score_rows=[_row("host-alpha", "2026-01-01T00:00:00Z", "pyod_ecod")]
    )

    score_row_composer.dump_score_rows(rows, output)
    persisted = json.loads(output.read_text(encoding="utf-8"))

    assert isinstance(persisted, list)
    assert persisted == rows


def test_cli_composes_repeated_score_row_sources(tmp_path: Path) -> None:
    base = tmp_path / "base.jsonl"
    native = tmp_path / "native.json"
    output = tmp_path / "composed-model-score-rows.json"
    base.write_text(
        json.dumps(_row("host-alpha", "2026-01-01T00:00:00Z", "pyod_ecod")) + "\n",
        encoding="utf-8",
    )
    native.write_text(
        json.dumps([_row("host-alpha", "2026-01-01T00:00:00Z", "stdlib_linear_native")]),
        encoding="utf-8",
    )

    assert (
        score_row_composer.main(
            [
                str(output),
                "--score-rows",
                str(base),
                "--score-rows",
                str(native),
            ]
        )
        == 0
    )
    persisted = json.loads(output.read_text(encoding="utf-8"))

    assert isinstance(persisted, list)
    assert list(persisted[0]["scores"]) == ["pyod_ecod", "stdlib_linear_native"]
