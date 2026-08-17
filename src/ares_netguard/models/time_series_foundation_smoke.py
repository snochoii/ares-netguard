"""Pinned real-model replay under an enforced Linux network namespace."""

from __future__ import annotations

import argparse
import json
import os
import resource
import shutil
import socket
import ssl
import subprocess
import sys
import time
from collections.abc import Mapping, Sequence
from pathlib import Path
from typing import Any, NoReturn

from ares_netguard.models.time_series_forecast import (
    CHRONOS_BACKEND_NAME,
    DEFAULT_BACKEND_NAME,
    ForecastEstimate,
    ForecastRequest,
    resolve_forecast_backend,
)
from ares_netguard.models.time_series_forecast_evaluation import (
    REPLAY_ANOMALY_COUNT,
    REPLAY_COHORT_SHA256,
    _canonical_sha256,
    _validate_label,
    dump_forecast_evaluation,
    generate_forecast_evaluation,
    generate_forecast_replay_evaluation,
    load_anomaly_labels,
    load_replay_anomaly_labels,
    validate_forecast_evaluation,
)
from ares_netguard.models.time_series_residual import (
    _atomic_write_text,
    dump_report,
    generate_residual_report,
    load_time_window_rows,
    validate_residual_report,
)

OPERATIONAL_SCHEMA_VERSION = "time_series_forecast_replay_operational.v0"
ANALYTICAL_SCHEMA_VERSION = "time_series_forecast_replay_analytical.v0"
ANALYTICAL_FIELDS = frozenset(
    {
        "schema_version",
        "cohort_sha256",
        "anomaly_labels",
        "proxy_residual_report",
        "chronos_residual_report",
        "evaluation",
    }
)
OPERATIONAL_FIELDS = frozenset(
    {
        "schema_version",
        "measurement_scope",
        "measurement_method",
        "runtime_attestation",
        "run_results",
        "reproducibility",
    }
)
MEASUREMENT_FIELDS = frozenset(
    {
        "clock",
        "latency_unit",
        "total_runtime_unit",
        "peak_rss_method",
        "peak_rss_unit",
    }
)
RUN_FIELDS = frozenset(
    {
        "run_id",
        "analytical_sha256",
        "backend_measurements",
        "total_replay_runtime_ns",
        "peak_rss_kib",
        "network_canary",
        "process_isolation",
    }
)
BACKEND_MEASUREMENT_FIELDS = frozenset({"backend_id", "inference_count", "inference_latency_ns"})
CANARY_FIELDS = frozenset(
    {
        "mechanism",
        "namespace_changed",
        "interfaces",
        "target",
        "external_connection_allowed",
        "failure_type",
        "failure_errno",
    }
)
PROCESS_ISOLATION_FIELDS = frozenset(
    {
        "os_level_subprocess_denial",
        "python_subprocess_tripwire_triggered",
        "expected_child_processes",
        "native_threads_allowed",
        "descendants_inherit_network_namespace",
    }
)
REPRODUCIBILITY_FIELDS = frozenset(
    {"run_count", "comparison_scope", "analytical_sha256", "exact_match"}
)
EXPECTED_INFERENCE_COUNT = 544
AUDIT_EVENTS = frozenset({"subprocess.Popen", "os.system", "os.posix_spawn", "os.fork"})
LOWER_HEX = frozenset("0123456789abcdef")


class _TimedBackend:
    def __init__(self, backend: Any) -> None:
        self._backend = backend
        self.inference_latency_ns: list[int] = []

    def __getattr__(self, name: str) -> Any:
        return getattr(self._backend, name)

    def forecast_one(self, request: ForecastRequest) -> ForecastEstimate:
        started = time.perf_counter_ns()
        result = self._backend.forecast_one(request)
        elapsed = time.perf_counter_ns() - started
        self.inference_latency_ns.append(elapsed)
        return result


def _forbidden_access(*_args: object, **_kwargs: object) -> NoReturn:
    raise RuntimeError("foundation replay denied network or Python subprocess access")


def _safe_output_dir(path: str | Path) -> Path:
    output = Path(path)
    resolved = output.resolve(strict=False)
    if not output.is_absolute() or not resolved.is_relative_to(Path("/tmp")):
        raise ValueError("foundation replay output_dir must be under /tmp")
    output.mkdir(parents=True, exist_ok=True)
    return output


def _network_canary(parent_netns: str) -> dict[str, object]:
    child_netns = os.readlink("/proc/self/ns/net")
    namespace_changed = child_netns != parent_netns
    interfaces = []
    for line in Path("/proc/net/dev").read_text(encoding="utf-8").splitlines()[2:]:
        if ":" in line:
            interfaces.append(line.split(":", 1)[0].strip())
    interfaces.sort()
    if not namespace_changed or interfaces != ["lo"]:
        raise RuntimeError("B1_REAL_MODEL_GATE: blocked: network namespace isolation failed")
    try:
        socket.create_connection(("1.1.1.1", 443), timeout=1.0)
    except OSError as exc:
        return {
            "mechanism": "linux_unshare_user_map_root_network_namespace",
            "namespace_changed": True,
            "interfaces": interfaces,
            "target": "1.1.1.1:443",
            "external_connection_allowed": False,
            "failure_type": type(exc).__name__,
            "failure_errno": exc.errno,
        }
    raise RuntimeError("B1_REAL_MODEL_GATE: blocked: external network canary succeeded")


def _install_runtime_guards() -> None:
    if ssl.SSLContext is None:  # pragma: no cover - defensive interpreter guard
        raise RuntimeError("SSL runtime is unavailable")

    def audit(event: str, _args: tuple[object, ...]) -> None:
        if event in AUDIT_EVENTS:
            raise RuntimeError(f"foundation replay subprocess audit tripwire: {event}")

    sys.addaudithook(audit)
    socket.socket = _forbidden_access  # type: ignore[assignment]
    socket.create_connection = _forbidden_access  # type: ignore[assignment]
    subprocess.Popen = _forbidden_access  # type: ignore[assignment,misc]
    subprocess.run = _forbidden_access  # type: ignore[assignment]


def _configure_determinism(backend: Any) -> None:
    torch = getattr(backend, "_torch", None)
    if torch is None:
        raise RuntimeError("Chronos backend did not expose the pinned torch runtime")
    torch.set_num_threads(1)
    torch.set_num_interop_threads(1)
    torch.manual_seed(0)
    torch.use_deterministic_algorithms(True)


def _validate_isolated_runtime_paths(isolation_root: Path, model_root: str | Path) -> None:
    resolved_model = Path(model_root).resolve()
    resolved_prefix = Path(sys.prefix).resolve()
    if not resolved_model.is_relative_to(isolation_root):
        raise ValueError("B1 replay model must be under the isolated root")
    if not resolved_prefix.is_relative_to(isolation_root):
        raise ValueError("B1 replay virtual environment must be under the isolated root")


def _backend_measurement(backend: _TimedBackend) -> dict[str, object]:
    return {
        "backend_id": backend.backend_id,
        "inference_count": len(backend.inference_latency_ns),
        "inference_latency_ns": list(backend.inference_latency_ns),
    }


def _analytical_evidence(
    *,
    proxy_report: Mapping[str, Any],
    chronos_report: Mapping[str, Any],
    labels: Sequence[Mapping[str, Any]],
    evaluation: Mapping[str, Any],
) -> dict[str, object]:
    evidence = {
        "schema_version": ANALYTICAL_SCHEMA_VERSION,
        "cohort_sha256": REPLAY_COHORT_SHA256,
        "anomaly_labels": [dict(label) for label in labels],
        "proxy_residual_report": dict(proxy_report),
        "chronos_residual_report": dict(chronos_report),
        "evaluation": dict(evaluation),
    }
    validate_analytical_evidence(evidence)
    return evidence


def _run_inside_namespace(
    *,
    windows_path: str | Path,
    labels_path: str | Path,
    replay_windows_path: str | Path,
    replay_labels_path: str | Path,
    model_root: str | Path,
    output_dir: str | Path,
    run_id: str,
    parent_netns: str,
) -> dict[str, object]:
    output = _safe_output_dir(output_dir)
    canary = _network_canary(parent_netns)
    os.environ.update(
        {
            "HF_HUB_OFFLINE": "1",
            "TRANSFORMERS_OFFLINE": "1",
            "HF_HUB_DISABLE_TELEMETRY": "1",
            "DO_NOT_TRACK": "1",
        }
    )
    _install_runtime_guards()

    replay_rows = load_time_window_rows(replay_windows_path)
    started = time.perf_counter_ns()
    chronos_backend = resolve_forecast_backend(CHRONOS_BACKEND_NAME, model_root=model_root)
    _configure_determinism(chronos_backend)
    proxy = _TimedBackend(resolve_forecast_backend(DEFAULT_BACKEND_NAME))
    chronos = _TimedBackend(chronos_backend)
    proxy_replay = generate_residual_report(
        replay_rows,
        history_window=64,
        calibration_window=32,
        backend=proxy,
    )
    chronos_replay = generate_residual_report(
        replay_rows,
        history_window=64,
        calibration_window=32,
        interval_z=2.0,
        backend=chronos,
    )
    replay_labels = load_replay_anomaly_labels(replay_labels_path)
    replay_evaluation = generate_forecast_replay_evaluation(
        proxy_replay,
        chronos_replay,
        replay_rows,
        replay_labels,
    )
    total_runtime_ns = time.perf_counter_ns() - started
    peak_rss_kib = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
    analytical = _analytical_evidence(
        proxy_report=proxy_replay,
        chronos_report=chronos_replay,
        labels=replay_labels,
        evaluation=replay_evaluation,
    )
    if str(model_root) in json.dumps(analytical, allow_nan=False):
        raise ValueError("foundation replay copied the local model path")

    # Preserve and revalidate the original v0 evidence contract inside the same isolation.
    rows = load_time_window_rows(windows_path)
    proxy_report = generate_residual_report(rows, history_window=64, calibration_window=32)
    chronos_report = generate_residual_report(
        rows,
        history_window=64,
        calibration_window=32,
        interval_z=2.0,
        backend=chronos_backend,
    )
    labels = load_anomaly_labels(labels_path)
    evaluation = generate_forecast_evaluation(proxy_report, chronos_report, rows, labels)

    dump_report(proxy_report, output / "proxy-residual-report.json")
    dump_report(chronos_report, output / "chronos-residual-report.json")
    dump_forecast_evaluation(evaluation, output / "forecast-evaluation.json")
    dump_report(proxy_replay, output / "proxy-replay-residual-report.json")
    dump_report(chronos_replay, output / "chronos-replay-residual-report.json")
    dump_forecast_evaluation(replay_evaluation, output / "replay-evaluation.json")
    _atomic_write_text(
        output / "analytical-evidence.json",
        json.dumps(analytical, allow_nan=False, indent=2, sort_keys=True) + "\n",
    )

    run_result = {
        "run_id": run_id,
        "analytical_sha256": _canonical_sha256(analytical),
        "backend_measurements": [_backend_measurement(chronos), _backend_measurement(proxy)],
        "total_replay_runtime_ns": total_runtime_ns,
        "peak_rss_kib": peak_rss_kib,
        "network_canary": canary,
        "process_isolation": {
            "os_level_subprocess_denial": False,
            "python_subprocess_tripwire_triggered": False,
            "expected_child_processes": 0,
            "native_threads_allowed": True,
            "descendants_inherit_network_namespace": True,
        },
    }
    _validate_run_result(run_result)
    _atomic_write_text(
        output / "operational-run.json",
        json.dumps(run_result, allow_nan=False, indent=2, sort_keys=True) + "\n",
    )
    return replay_evaluation


def _isolated_environment(isolation_root: Path) -> dict[str, str]:
    cache = isolation_root / "cache"
    values = {
        "HOME": cache / "home",
        "XDG_CACHE_HOME": cache / "xdg",
        "PIP_CACHE_DIR": cache / "pip",
        "HF_HOME": cache / "hf",
        "HF_HUB_CACHE": cache / "hf-hub",
        "HUGGINGFACE_HUB_CACHE": cache / "hf-hub",
        "HF_XET_CACHE": cache / "hf-xet",
        "HF_ASSETS_CACHE": cache / "hf-assets",
        "HF_MODULES_CACHE": cache / "hf-modules",
        "TRANSFORMERS_CACHE": cache / "transformers",
        "TORCH_HOME": cache / "torch",
        "TORCH_EXTENSIONS_DIR": cache / "torch-extensions",
        "TORCHINDUCTOR_CACHE_DIR": cache / "torchinductor",
        "TRITON_CACHE_DIR": cache / "triton",
        "MPLCONFIGDIR": cache / "matplotlib",
        "PYTHONPYCACHEPREFIX": cache / "pycache",
        "TMPDIR": cache / "tmp",
    }
    for path in values.values():
        path.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ)
    env.update({key: str(value) for key, value in values.items()})
    env.update(
        {
            "PYTHONNOUSERSITE": "1",
            "PYTHONPATH": str((Path.cwd() / "src").resolve()),
            "PIP_CONFIG_FILE": "/dev/null",
            "PIP_DISABLE_PIP_VERSION_CHECK": "1",
            "PYTHONHASHSEED": "0",
            "OMP_NUM_THREADS": "1",
            "MKL_NUM_THREADS": "1",
            "OPENBLAS_NUM_THREADS": "1",
            "NUMEXPR_NUM_THREADS": "1",
            "TOKENIZERS_PARALLELISM": "false",
            "HF_HUB_OFFLINE": "1",
            "TRANSFORMERS_OFFLINE": "1",
            "HF_HUB_DISABLE_TELEMETRY": "1",
            "DO_NOT_TRACK": "1",
        }
    )
    return env


def run_smoke(
    windows_path: str | Path,
    labels_path: str | Path,
    replay_windows_path: str | Path,
    replay_labels_path: str | Path,
    model_root: str | Path,
    output_dir: str | Path,
    environment_attestation_path: str | Path,
) -> dict[str, object]:
    """Launch two independent, OS-network-denied replay processes and compare them."""
    output = _safe_output_dir(output_dir)
    isolation_root = output.parent.resolve()
    _validate_isolated_runtime_paths(isolation_root, model_root)
    attestation_path = Path(environment_attestation_path).resolve()
    if not attestation_path.is_relative_to(isolation_root):
        raise ValueError("B1 environment attestation must be under the isolated root")
    attestation = json.loads(attestation_path.read_text(encoding="utf-8"))
    parent_netns = os.readlink("/proc/self/ns/net")
    env = _isolated_environment(isolation_root)

    run_dirs = []
    for run_number in (1, 2):
        run_id = f"run-{run_number}"
        run_dir = output / run_id
        command = [
            "/usr/bin/unshare",
            "--user",
            "--map-root-user",
            "--net",
            sys.executable,
            "-m",
            "ares_netguard.models.time_series_foundation_smoke",
            str(windows_path),
            str(labels_path),
            str(model_root),
            str(run_dir),
            "--replay-windows",
            str(replay_windows_path),
            "--replay-labels",
            str(replay_labels_path),
            "--inner-run",
            run_id,
            "--parent-netns",
            parent_netns,
        ]
        try:
            subprocess.run(command, check=True, env=env)
        except (OSError, subprocess.CalledProcessError) as exc:
            raise RuntimeError("B1_REAL_MODEL_GATE: blocked") from exc
        run_dirs.append(run_dir)

    analytical_reports = [
        json.loads((run_dir / "analytical-evidence.json").read_text(encoding="utf-8"))
        for run_dir in run_dirs
    ]
    for report in analytical_reports:
        validate_analytical_evidence(report)
    if analytical_reports[0] != analytical_reports[1]:
        raise RuntimeError("B1_REAL_MODEL_GATE: blocked: analytical replay is not exact")
    analytical_sha = _canonical_sha256(analytical_reports[0])
    run_results = [
        json.loads((run_dir / "operational-run.json").read_text(encoding="utf-8"))
        for run_dir in run_dirs
    ]
    operational = {
        "schema_version": OPERATIONAL_SCHEMA_VERSION,
        "measurement_scope": "two_independent_pinned_cpu_replay_processes",
        "measurement_method": {
            "clock": "time.perf_counter_ns",
            "latency_unit": "nanoseconds",
            "total_runtime_unit": "nanoseconds",
            "peak_rss_method": "resource.getrusage_RUSAGE_SELF_ru_maxrss",
            "peak_rss_unit": "KiB",
        },
        "runtime_attestation": attestation,
        "run_results": run_results,
        "reproducibility": {
            "run_count": 2,
            "comparison_scope": "deterministic_analytical_only",
            "analytical_sha256": [analytical_sha, analytical_sha],
            "exact_match": True,
        },
    }
    validate_operational_evidence(operational)
    _atomic_write_text(
        output / "operational-evidence.json",
        json.dumps(operational, allow_nan=False, indent=2, sort_keys=True) + "\n",
    )
    for filename in (
        "analytical-evidence.json",
        "forecast-evaluation.json",
        "replay-evaluation.json",
    ):
        shutil.copyfile(run_dirs[0] / filename, output / filename)
    print("PRIMARY_REPLAY: valid")
    print("STRESS_CASES: 2/2 rejected_as_expected")
    print("ANALYTICAL_REPRODUCIBILITY: exact")
    print("OPERATIONAL_EVIDENCE: valid")
    print("OS_NETWORK_DENIAL: verified")
    print("B1_REAL_MODEL_GATE: ready")
    return analytical_reports[0]


def _exact_fields(raw: Mapping[str, Any], expected: frozenset[str], field: str) -> None:
    if set(raw) != expected:
        raise ValueError(f"{field} fields are invalid")


def validate_analytical_evidence(report: Mapping[str, Any]) -> None:
    _exact_fields(report, ANALYTICAL_FIELDS, "analytical evidence")
    if (
        report["schema_version"] != ANALYTICAL_SCHEMA_VERSION
        or report["cohort_sha256"] != REPLAY_COHORT_SHA256
    ):
        raise ValueError("analytical evidence identity is invalid")
    labels = report["anomaly_labels"]
    if not isinstance(labels, list) or len(labels) != REPLAY_ANOMALY_COUNT:
        raise ValueError("analytical evidence anomaly labels are invalid")
    normalized_labels = [_validate_label(label) for label in labels]
    label_keys = [
        (label["entity_id"], label["feature_name"], label["window_start"])
        for label in normalized_labels
    ]
    if label_keys != sorted(label_keys) or len(label_keys) != len(set(label_keys)):
        raise ValueError("analytical evidence anomaly labels must be sorted and unique")
    for field in ("proxy_residual_report", "chronos_residual_report"):
        residual_report = report[field]
        if not isinstance(residual_report, Mapping):
            raise ValueError(f"analytical evidence {field} must be an object")
        validate_residual_report(residual_report)
    evaluation = report["evaluation"]
    if not isinstance(evaluation, Mapping):
        raise ValueError("analytical evidence evaluation must be an object")
    validate_forecast_evaluation(evaluation)


def _non_negative_integer(value: Any, field: str, *, positive: bool = False) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or value < int(positive):
        raise ValueError(f"{field} must be a {'positive' if positive else 'non-negative'} integer")
    return value


def _is_sha256(value: Any) -> bool:
    return isinstance(value, str) and len(value) == 64 and set(value) <= LOWER_HEX


def _validate_run_result(run: Any) -> None:
    if not isinstance(run, Mapping):
        raise ValueError("operational run result must be an object")
    _exact_fields(run, RUN_FIELDS, "operational run")
    if run["run_id"] not in {"run-1", "run-2"}:
        raise ValueError("operational run_id is invalid")
    if not _is_sha256(run["analytical_sha256"]):
        raise ValueError("operational analytical_sha256 is invalid")
    measurements = run["backend_measurements"]
    if not isinstance(measurements, list) or len(measurements) != 2:
        raise ValueError("operational backend measurements are invalid")
    expected_ids = ["chronos_bolt_tiny_local_v1", "rolling_mean_proxy_v1"]
    for measurement, backend_id in zip(measurements, expected_ids, strict=True):
        if not isinstance(measurement, Mapping):
            raise ValueError("operational backend measurement must be an object")
        _exact_fields(measurement, BACKEND_MEASUREMENT_FIELDS, "backend measurement")
        count = _non_negative_integer(measurement["inference_count"], "inference_count")
        latencies = measurement["inference_latency_ns"]
        if measurement["backend_id"] != backend_id or count != EXPECTED_INFERENCE_COUNT:
            raise ValueError("operational backend measurement contract drifted")
        if not isinstance(latencies, list) or len(latencies) != count:
            raise ValueError("operational latency sample count is invalid")
        for latency in latencies:
            _non_negative_integer(latency, "inference latency")
    _non_negative_integer(run["total_replay_runtime_ns"], "total replay runtime", positive=True)
    _non_negative_integer(run["peak_rss_kib"], "peak RSS", positive=True)
    canary = run["network_canary"]
    if not isinstance(canary, Mapping):
        raise ValueError("network canary must be an object")
    _exact_fields(canary, CANARY_FIELDS, "network canary")
    if (
        canary["mechanism"] != "linux_unshare_user_map_root_network_namespace"
        or canary["namespace_changed"] is not True
        or canary["interfaces"] != ["lo"]
        or canary["target"] != "1.1.1.1:443"
        or canary["external_connection_allowed"] is not False
        or not isinstance(canary["failure_type"], str)
    ):
        raise ValueError("network canary is invalid")
    if canary["failure_errno"] is not None:
        _non_negative_integer(canary["failure_errno"], "network canary errno", positive=True)
    isolation = run["process_isolation"]
    if not isinstance(isolation, Mapping):
        raise ValueError("process isolation must be an object")
    _exact_fields(isolation, PROCESS_ISOLATION_FIELDS, "process isolation")
    expected_isolation = {
        "os_level_subprocess_denial": False,
        "python_subprocess_tripwire_triggered": False,
        "expected_child_processes": 0,
        "native_threads_allowed": True,
        "descendants_inherit_network_namespace": True,
    }
    if dict(isolation) != expected_isolation:
        raise ValueError("process isolation is invalid")


def validate_operational_evidence(report: Mapping[str, Any]) -> None:
    _exact_fields(report, OPERATIONAL_FIELDS, "operational evidence")
    if report["schema_version"] != OPERATIONAL_SCHEMA_VERSION:
        raise ValueError("operational evidence schema is invalid")
    if report["measurement_scope"] != "two_independent_pinned_cpu_replay_processes":
        raise ValueError("operational measurement scope is invalid")
    method = report["measurement_method"]
    if not isinstance(method, Mapping):
        raise ValueError("operational measurement method must be an object")
    _exact_fields(method, MEASUREMENT_FIELDS, "measurement method")
    expected_method = {
        "clock": "time.perf_counter_ns",
        "latency_unit": "nanoseconds",
        "total_runtime_unit": "nanoseconds",
        "peak_rss_method": "resource.getrusage_RUSAGE_SELF_ru_maxrss",
        "peak_rss_unit": "KiB",
    }
    if dict(method) != expected_method:
        raise ValueError("operational measurement method is invalid")
    if not isinstance(report["runtime_attestation"], Mapping):
        raise ValueError("runtime attestation must be an object")
    runs = report["run_results"]
    if not isinstance(runs, list) or len(runs) != 2:
        raise ValueError("operational evidence requires two runs")
    for run in runs:
        _validate_run_result(run)
    reproducibility = report["reproducibility"]
    if not isinstance(reproducibility, Mapping):
        raise ValueError("operational reproducibility must be an object")
    _exact_fields(reproducibility, REPRODUCIBILITY_FIELDS, "operational reproducibility")
    hashes = reproducibility["analytical_sha256"]
    if (
        reproducibility["run_count"] != 2
        or reproducibility["comparison_scope"] != "deterministic_analytical_only"
        or reproducibility["exact_match"] is not True
        or not isinstance(hashes, list)
        or len(hashes) != 2
        or hashes[0] != hashes[1]
        or not all(_is_sha256(item) for item in hashes)
        or hashes != [run["analytical_sha256"] for run in runs]
    ):
        raise ValueError("operational reproducibility is invalid")


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Run the pinned Chronos backend offline.")
    parser.add_argument("windows", help="strict v0 synthetic time-window JSONL fixture")
    parser.add_argument("labels", help="strict v0 synthetic anomaly-label JSONL fixture")
    parser.add_argument("model_root", help="explicit verified local model root")
    parser.add_argument("output_dir", help="absolute /tmp output directory")
    parser.add_argument("--replay-windows", required=True)
    parser.add_argument("--replay-labels", required=True)
    parser.add_argument("--environment-attestation")
    parser.add_argument("--inner-run", choices=("run-1", "run-2"))
    parser.add_argument("--parent-netns")
    args = parser.parse_args(argv)

    if args.inner_run:
        if not args.parent_netns:
            raise SystemExit("B1_REAL_MODEL_GATE: blocked: missing parent network namespace")
        _run_inside_namespace(
            windows_path=args.windows,
            labels_path=args.labels,
            replay_windows_path=args.replay_windows,
            replay_labels_path=args.replay_labels,
            model_root=args.model_root,
            output_dir=args.output_dir,
            run_id=args.inner_run,
            parent_netns=args.parent_netns,
        )
        return 0
    if not args.environment_attestation:
        raise SystemExit("B1_REAL_MODEL_GATE: blocked: missing environment attestation")
    run_smoke(
        args.windows,
        args.labels,
        args.replay_windows,
        args.replay_labels,
        args.model_root,
        args.output_dir,
        args.environment_attestation,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
