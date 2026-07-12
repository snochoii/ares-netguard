"""Explicit real-model smoke lane with network and subprocess access denied."""

from __future__ import annotations

import argparse
import json
import os
import socket
import ssl
import subprocess
from collections.abc import Sequence
from pathlib import Path
from typing import NoReturn

from ares_netguard.models.time_series_forecast import (
    CHRONOS_BACKEND_NAME,
    resolve_forecast_backend,
)
from ares_netguard.models.time_series_forecast_evaluation import (
    dump_forecast_evaluation,
    generate_forecast_evaluation,
    load_anomaly_labels,
)
from ares_netguard.models.time_series_residual import (
    dump_report,
    generate_residual_report,
    load_time_window_rows,
)


def _forbidden_access(*_args: object, **_kwargs: object) -> NoReturn:
    raise RuntimeError("foundation forecast smoke denied network or subprocess access")


def run_smoke(
    windows_path: str | Path,
    labels_path: str | Path,
    model_root: str | Path,
    output_dir: str | Path,
) -> dict[str, object]:
    """Run proxy, repeated Chronos inference, and held-out comparison offline."""
    output = Path(output_dir)
    if not output.is_absolute() or not output.resolve(strict=False).is_relative_to(Path("/tmp")):
        raise ValueError("foundation forecast smoke output_dir must be under /tmp")
    output.mkdir(parents=True, exist_ok=True)

    os.environ["HF_HUB_OFFLINE"] = "1"
    os.environ["TRANSFORMERS_OFFLINE"] = "1"
    os.environ["HF_HUB_DISABLE_TELEMETRY"] = "1"
    os.environ["DO_NOT_TRACK"] = "1"
    # Import SSL before replacing socket.socket: Python's ssl module subclasses
    # the original socket type during import, while later connection attempts
    # still reach the denied socket constructor.
    if ssl.SSLContext is None:  # pragma: no cover - defensive interpreter guard
        raise RuntimeError("SSL runtime is unavailable")
    socket.socket = _forbidden_access  # type: ignore[assignment]
    socket.create_connection = _forbidden_access  # type: ignore[assignment]
    subprocess.Popen = _forbidden_access  # type: ignore[assignment,misc]
    subprocess.run = _forbidden_access  # type: ignore[assignment]

    rows = load_time_window_rows(windows_path)
    proxy_report = generate_residual_report(rows, history_window=64, calibration_window=32)
    backend = resolve_forecast_backend(CHRONOS_BACKEND_NAME, model_root=model_root)
    first = generate_residual_report(
        rows,
        history_window=64,
        calibration_window=32,
        interval_z=2.0,
        backend=backend,
    )
    second = generate_residual_report(
        rows,
        history_window=64,
        calibration_window=32,
        interval_z=2.0,
        backend=backend,
    )
    if first != second:
        raise ValueError("foundation forecast smoke is not deterministic")
    if str(model_root) in json.dumps(first, allow_nan=False):
        raise ValueError("foundation forecast report copied the local model path")

    labels = load_anomaly_labels(labels_path)
    evaluation = generate_forecast_evaluation(proxy_report, first, rows, labels)
    dump_report(proxy_report, output / "proxy-residual-report.json")
    dump_report(first, output / "chronos-residual-report.json")
    dump_forecast_evaluation(evaluation, output / "forecast-evaluation.json")
    return evaluation


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Run the pinned Chronos backend fully offline.")
    parser.add_argument("windows", help="strict synthetic time-window JSONL fixture")
    parser.add_argument("labels", help="strict synthetic anomaly-label JSONL fixture")
    parser.add_argument("model_root", help="explicit verified local model root")
    parser.add_argument("output_dir", help="absolute /tmp output directory")
    args = parser.parse_args(argv)

    run_smoke(args.windows, args.labels, args.model_root, args.output_dir)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
