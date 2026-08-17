from __future__ import annotations

import hashlib
import importlib
import json
import os
import socket
import subprocess
import sys
import types
from contextlib import nullcontext
from dataclasses import FrozenInstanceError
from pathlib import Path
from types import MappingProxyType

import pytest

from ares_netguard.models import time_series_forecast as forecast
from ares_netguard.models import time_series_foundation_smoke as foundation_smoke
from ares_netguard.models.time_series_forecast import (
    CHRONOS_BACKEND_NAME,
    CHRONOS_PACKAGE_VERSIONS,
    DEFAULT_BACKEND_NAME,
    OFFLINE_SYNTHETIC_BACKEND_SAFETY,
    ForecastBackend,
    ForecastEstimate,
    ForecastRequest,
    RollingMeanProxyBackend,
    resolve_forecast_backend,
    validate_forecast_estimate,
)


class _FakeTorch:
    float32 = "float32"

    @staticmethod
    def tensor(values: object, *, dtype: object) -> object:
        assert dtype == "float32"
        return values

    @staticmethod
    def inference_mode() -> object:
        return nullcontext()


class _FakePipeline:
    def __init__(self) -> None:
        self.contexts: list[tuple[float, ...]] = []

    def predict_quantiles(
        self,
        context: tuple[float, ...],
        *,
        prediction_length: int,
        quantile_levels: list[float],
    ) -> tuple[list[list[list[float]]], list[list[float]]]:
        self.contexts.append(tuple(context))
        assert prediction_length == 1
        assert quantile_levels == [0.1, 0.5, 0.9]
        return [[[8.0, 10.0, 12.0]]], [[999.0]]


def _fixture_manifest(config: bytes, weights: bytes) -> forecast._ChronosManifest:
    return forecast._ChronosManifest(
        model_id=forecast.CHRONOS_MODEL_ID,
        revision=forecast.CHRONOS_MODEL_REVISION,
        license_id="apache-2.0",
        serialization="safetensors",
        runtime_platform=forecast.CHRONOS_RUNTIME_PLATFORM,
        packages=CHRONOS_PACKAGE_VERSIONS,
        files=(
            forecast._ManifestFile(
                name="config.json",
                size_bytes=len(config),
                sha256=hashlib.sha256(config).hexdigest(),
            ),
            forecast._ManifestFile(
                name="model.safetensors",
                size_bytes=len(weights),
                sha256=hashlib.sha256(weights).hexdigest(),
            ),
        ),
    )


def _fixture_bundle(tmp_path: Path) -> tuple[Path, forecast._ChronosManifest]:
    root = tmp_path / "chronos-model"
    root.mkdir(parents=True)
    config = json.dumps(
        {
            "architectures": ["ChronosBoltModelForForecasting"],
            "chronos_pipeline_class": "ChronosBoltPipeline",
            "chronos_config": {
                "input_patch_size": 16,
                "prediction_length": 64,
                "quantiles": [index / 10 for index in range(1, 10)],
            },
        },
        sort_keys=True,
    ).encode()
    weights = b"safe synthetic safetensors stand-in"
    (root / "config.json").write_bytes(config)
    (root / "model.safetensors").write_bytes(weights)
    return root, _fixture_manifest(config, weights)


def _operational_run(run_id: str, *, latency: int) -> dict[str, object]:
    def measurement(backend_id: str) -> dict[str, object]:
        return {
            "backend_id": backend_id,
            "inference_count": foundation_smoke.EXPECTED_INFERENCE_COUNT,
            "inference_latency_ns": [latency] * foundation_smoke.EXPECTED_INFERENCE_COUNT,
        }

    return {
        "run_id": run_id,
        "analytical_sha256": "a" * 64,
        "backend_measurements": [
            measurement("chronos_bolt_tiny_local_v1"),
            measurement("rolling_mean_proxy_v1"),
        ],
        "total_replay_runtime_ns": latency * 2000,
        "peak_rss_kib": 1000 + latency,
        "network_canary": {
            "mechanism": "linux_unshare_user_map_root_network_namespace",
            "namespace_changed": True,
            "interfaces": ["lo"],
            "target": "1.1.1.1:443",
            "external_connection_allowed": False,
            "failure_type": "PermissionError",
            "failure_errno": 1,
        },
        "process_isolation": {
            "os_level_subprocess_denial": False,
            "python_subprocess_tripwire_triggered": False,
            "expected_child_processes": 0,
            "native_threads_allowed": True,
            "descendants_inherit_network_namespace": True,
        },
    }


def _runtime_attestation() -> dict[str, object]:
    return {
        "schema_version": "foundation_forecast_environment_attestation.v0",
        "source_identities": dict(foundation_smoke.EXPECTED_SOURCE_IDENTITIES),
        "python": "3.12.3",
        "runtime_platform": "cpython312_linux_x86_64_cpu",
        "package_count": 41,
        "package_set_sha256": ("065eb94ed00987015e097f936a19175cab763e99d3d62848c18000eb70fcef5b"),
        "wheel_count": 41,
        "wheel_set_sha256": ("deb8a9616a2648c468ae096a867d881f32ec24bba00c0e7323af887895a7bf5b"),
        "model_id": "amazon/chronos-bolt-tiny",
        "model_revision": "a0e552de83495b5c28c14c71c374f3e33280b340",
        "model_files": json.loads(json.dumps(foundation_smoke.EXPECTED_MODEL_FILES)),
        "model_bundle_sha256": ("9aefb3d869a0a475c81a8685ffaeb99c63a3e3ee1cfc03befeb7cd47b58c00ab"),
        "pip_check_passed": True,
    }


def _operational_evidence() -> dict[str, object]:
    runs = [_operational_run("run-1", latency=10), _operational_run("run-2", latency=20)]
    return {
        "schema_version": foundation_smoke.OPERATIONAL_SCHEMA_VERSION,
        "measurement_scope": "two_independent_pinned_cpu_replay_processes",
        "measurement_method": {
            "clock": "time.perf_counter_ns",
            "latency_unit": "nanoseconds",
            "total_runtime_unit": "nanoseconds",
            "peak_rss_method": "resource.getrusage_RUSAGE_SELF_ru_maxrss",
            "peak_rss_unit": "KiB",
        },
        "runtime_attestation": _runtime_attestation(),
        "run_results": runs,
        "reproducibility": {
            "run_count": 2,
            "comparison_scope": "deterministic_analytical_only",
            "analytical_sha256": ["a" * 64, "a" * 64],
            "exact_match": True,
        },
    }


def test_forecast_contract_and_builtin_identity_are_fixed_and_immutable() -> None:
    backend = resolve_forecast_backend(DEFAULT_BACKEND_NAME)

    assert isinstance(backend, ForecastBackend)
    assert isinstance(backend, RollingMeanProxyBackend)
    assert backend.backend_id == "rolling_mean_proxy_v1"
    assert backend.backend_version == "1"
    assert backend.backend_kind == "deterministic_proxy"
    assert backend.safety is OFFLINE_SYNTHETIC_BACKEND_SAFETY
    assert backend.safety.local_only is True
    assert backend.safety.no_network is True
    assert dict(backend.settings) == {
        "mean_method": "context_arithmetic_mean",
        "minimum_scale": 1e-6,
        "scale_method": "context_population_standard_deviation",
    }

    with pytest.raises(TypeError):
        backend.settings["minimum_scale"] = 1.0  # type: ignore[index]
    with pytest.raises(FrozenInstanceError):
        ForecastRequest((1.0,), 2.0).interval_z = 3.0  # type: ignore[misc]


def test_v1_shaped_backend_remains_a_runtime_protocol_match() -> None:
    class V1ShapedBackend:
        backend_id = "fixture_v1_backend"
        backend_version = "1"
        backend_kind = "deterministic_proxy"
        safety = OFFLINE_SYNTHETIC_BACKEND_SAFETY

        @property
        def settings(self) -> MappingProxyType[str, object]:
            return MappingProxyType({"mean_method": "context_arithmetic_mean"})

        def forecast_one(self, request: ForecastRequest) -> ForecastEstimate:
            return ForecastEstimate(
                mean=request.context[-1],
                lower=request.context[-1] - 1.0,
                upper=request.context[-1] + 1.0,
                scale=1.0,
            )

    backend = V1ShapedBackend()

    assert isinstance(backend, ForecastBackend)
    assert not hasattr(backend, "artifact")
    assert not hasattr(backend, "run_requirements")


def test_rolling_mean_proxy_uses_context_population_scale() -> None:
    estimate = RollingMeanProxyBackend().forecast_one(
        ForecastRequest(context=(10.0, 12.0, 14.0), interval_z=2.0)
    )

    assert estimate.mean == 12.0
    assert estimate.scale == pytest.approx(1.632993161855452)
    assert estimate.lower == pytest.approx(8.734013676289095)
    assert estimate.upper == pytest.approx(15.265986323710905)


@pytest.mark.parametrize(
    "selector",
    [
        "",
        "unknown",
        "https://models.example/backend",
        "package.module.Backend",
        "organization/model",
        "/tmp/backend",
        "../backend",
    ],
)
def test_backend_resolver_is_closed_and_has_no_fallback(selector: str) -> None:
    with pytest.raises(ValueError, match="unsupported forecast backend"):
        resolve_forecast_backend(selector)


@pytest.mark.parametrize(
    "estimate, message",
    [
        (ForecastEstimate(0.0, -1.0, 1.0, 0.0), "scale must be positive"),
        (ForecastEstimate(0.0, 1.0, 2.0, 1.0), "lower <= mean <= upper"),
        (ForecastEstimate(float("nan"), -1.0, 1.0, 1.0), "mean must be a finite"),
        (ForecastEstimate(0.0, -1.0, float("inf"), 1.0), "upper must be a finite"),
    ],
)
def test_forecast_estimate_validation_is_fail_closed(
    estimate: ForecastEstimate, message: str
) -> None:
    with pytest.raises(ValueError, match=message):
        validate_forecast_estimate(estimate)


def test_builtin_forecast_needs_no_import_socket_or_process_access(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    def forbidden(*_args: object, **_kwargs: object) -> None:
        raise AssertionError("offline backend attempted forbidden access")

    monkeypatch.setattr(importlib, "import_module", forbidden)
    monkeypatch.setattr(socket, "socket", forbidden)
    monkeypatch.setattr(subprocess, "run", forbidden)

    estimate = resolve_forecast_backend(DEFAULT_BACKEND_NAME).forecast_one(
        ForecastRequest(context=(1.0, 2.0, 3.0), interval_z=2.0)
    )

    assert estimate.mean == 2.0
    assert estimate.scale > 0.0


def test_proxy_rejects_model_root_and_chronos_requires_one(tmp_path: Path) -> None:
    with pytest.raises(ValueError, match="does not accept a model root"):
        resolve_forecast_backend(DEFAULT_BACKEND_NAME, model_root=tmp_path)
    with pytest.raises(ValueError, match="requires an explicit model root"):
        resolve_forecast_backend(CHRONOS_BACKEND_NAME)


def test_pinned_manifest_has_exact_identity_and_runtime() -> None:
    manifest = forecast._load_chronos_manifest()

    assert manifest.model_id == "amazon/chronos-bolt-tiny"
    assert manifest.revision == "a0e552de83495b5c28c14c71c374f3e33280b340"
    assert dict(manifest.packages) == dict(CHRONOS_PACKAGE_VERSIONS)
    assert {item.name for item in manifest.files} == {"config.json", "model.safetensors"}


def test_optional_lock_matches_manifest_roots_and_artifact_policy() -> None:
    lock = Path("requirements-foundation-forecast.lock").read_text(encoding="utf-8")
    for package, version in CHRONOS_PACKAGE_VERSIONS.items():
        assert f"{package}=={version}" in lock
    assert "setuptools==" in lock
    assert "--hash=sha256:" in lock
    assert "# WARNING:" not in lock

    assert "*.safetensors" in Path(".gitignore").read_text(encoding="utf-8")
    assert "*.safetensors" in Path("AGENTS.md").read_text(encoding="utf-8")
    assert "*.safetensors" in Path("scripts/check_no_generated_artifacts.sh").read_text(
        encoding="utf-8"
    )


def test_local_chronos_backend_verifies_bundle_and_maps_quantiles(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    root, manifest = _fixture_bundle(tmp_path)
    pipeline = _FakePipeline()
    monkeypatch.setattr(forecast, "_load_chronos_manifest", lambda: manifest)
    monkeypatch.setattr(
        forecast,
        "_load_chronos_runtime",
        lambda _bundle: (pipeline, _FakeTorch(), dict(CHRONOS_PACKAGE_VERSIONS)),
    )

    backend = resolve_forecast_backend(CHRONOS_BACKEND_NAME, model_root=root)
    estimate = backend.forecast_one(
        ForecastRequest(context=tuple(float(index) for index in range(64)), interval_z=2.0)
    )

    assert backend.backend_id == "chronos_bolt_tiny_local_v1"
    assert isinstance(backend, forecast.PretrainedForecastBackend)
    assert backend.run_requirements.history_window == 64
    assert backend.artifact is not None
    assert backend.artifact.model_id == "amazon/chronos-bolt-tiny"
    assert estimate.mean == 10.0
    assert estimate.scale == pytest.approx(4.0 / 2.5631031310892007)
    assert estimate.lower == pytest.approx(10.0 - 2.0 * estimate.scale)
    assert estimate.upper == pytest.approx(10.0 + 2.0 * estimate.scale)
    assert pipeline.contexts == [tuple(float(index) for index in range(64))]


def test_bundle_preflight_rejects_digest_drift_and_unexpected_files(tmp_path: Path) -> None:
    root, manifest = _fixture_bundle(tmp_path)
    (root / "model.safetensors").write_bytes(b"drift")
    with pytest.raises(ValueError, match="file size does not match"):
        forecast._verify_model_bundle(root, manifest)

    root, manifest = _fixture_bundle(tmp_path / "second")
    (root / "README.md").write_text("unexpected", encoding="utf-8")
    with pytest.raises(ValueError, match="exactly the pinned files"):
        forecast._verify_model_bundle(root, manifest)


def test_bundle_preflight_rejects_symlinks(tmp_path: Path) -> None:
    root, manifest = _fixture_bundle(tmp_path)
    weights = root / "model.safetensors"
    target = tmp_path / "outside.safetensors"
    target.write_bytes(weights.read_bytes())
    weights.unlink()
    weights.symlink_to(target)

    with pytest.raises(ValueError, match="regular non-symlink files"):
        forecast._verify_model_bundle(root, manifest)


def test_bundle_preflight_rejects_unsafe_owner_and_write_permissions(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    root, manifest = _fixture_bundle(tmp_path)
    root.chmod(0o770)
    with pytest.raises(ValueError, match="must not be group- or other-writable"):
        forecast._verify_model_bundle(root, manifest)

    root.chmod(0o700)
    (root / "model.safetensors").chmod(0o666)
    with pytest.raises(ValueError, match="must not be group- or other-writable"):
        forecast._verify_model_bundle(root, manifest)

    (root / "model.safetensors").chmod(0o600)
    current_euid = os.geteuid()
    monkeypatch.setattr(forecast.os, "geteuid", lambda: current_euid + 1)
    with pytest.raises(ValueError, match="must be owned by the current user"):
        forecast._verify_model_bundle(root, manifest)


def test_bundle_preflight_rejects_special_files_and_remote_code_config(tmp_path: Path) -> None:
    root, manifest = _fixture_bundle(tmp_path)
    weights = root / "model.safetensors"
    weights.unlink()
    os.mkfifo(weights)
    with pytest.raises(ValueError, match="regular non-symlink files"):
        forecast._verify_model_bundle(root, manifest)

    remote_root = tmp_path / "remote-code" / "chronos-model"
    remote_root.mkdir(parents=True)
    config = json.dumps(
        {
            "architectures": ["ChronosBoltModelForForecasting"],
            "auto_map": {"AutoModel": "custom.Model"},
            "chronos_pipeline_class": "ChronosBoltPipeline",
            "chronos_config": {
                "input_patch_size": 16,
                "prediction_length": 64,
                "quantiles": [index / 10 for index in range(1, 10)],
            },
        },
        sort_keys=True,
    ).encode()
    weights_bytes = b"safe synthetic safetensors stand-in"
    (remote_root / "config.json").write_bytes(config)
    (remote_root / "model.safetensors").write_bytes(weights_bytes)
    remote_manifest = _fixture_manifest(config, weights_bytes)
    with pytest.raises(ValueError, match="remote-code mapping is forbidden"):
        forecast._verify_model_bundle(remote_root, remote_manifest)


def test_backend_rechecks_bundle_after_loading(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    root, manifest = _fixture_bundle(tmp_path)
    monkeypatch.setattr(forecast, "_load_chronos_manifest", lambda: manifest)

    def mutate_during_load(
        _bundle: forecast._VerifiedBundle,
    ) -> tuple[object, object, dict[str, str]]:
        weights = root / "model.safetensors"
        original = weights.read_bytes()
        weights.write_bytes(b"x" * len(original))
        return _FakePipeline(), _FakeTorch(), dict(CHRONOS_PACKAGE_VERSIONS)

    monkeypatch.setattr(forecast, "_load_chronos_runtime", mutate_during_load)

    with pytest.raises(ValueError, match="file digest does not match"):
        resolve_forecast_backend(CHRONOS_BACKEND_NAME, model_root=root)


def test_chronos_runtime_uses_only_pinned_local_loader_arguments(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    root, manifest = _fixture_bundle(tmp_path)
    bundle = forecast._verify_model_bundle(root, manifest)
    calls: dict[str, object] = {}

    class FakeModel:
        def eval(self) -> None:
            calls["eval"] = True

    class FakeBaseChronosPipeline:
        @classmethod
        def from_pretrained(cls, path: Path, **kwargs: object) -> object:
            calls["path"] = path
            calls["kwargs"] = kwargs
            return types.SimpleNamespace(model=FakeModel())

    torch_module = types.ModuleType("torch")
    torch_module.float32 = "float32"  # type: ignore[attr-defined]
    torch_module.manual_seed = lambda seed: calls.setdefault("seed", seed)  # type: ignore[attr-defined]
    torch_module.use_deterministic_algorithms = (  # type: ignore[attr-defined]
        lambda enabled: calls.setdefault("deterministic", enabled)
    )
    torch_module.set_num_threads = lambda count: calls.setdefault("threads", count)  # type: ignore[attr-defined]
    chronos_module = types.ModuleType("chronos")
    chronos_module.BaseChronosPipeline = FakeBaseChronosPipeline  # type: ignore[attr-defined]

    monkeypatch.setitem(sys.modules, "torch", torch_module)
    monkeypatch.setitem(sys.modules, "chronos", chronos_module)
    monkeypatch.setattr(
        forecast,
        "_runtime_package_versions",
        lambda expected: dict(expected),
    )
    monkeypatch.setattr(socket, "socket", lambda *_args, **_kwargs: pytest.fail("network used"))
    monkeypatch.setattr(
        subprocess,
        "run",
        lambda *_args, **_kwargs: pytest.fail("subprocess used"),
    )

    _pipeline, _torch, versions = forecast._load_chronos_runtime(bundle)

    assert calls["path"] == root.resolve()
    assert calls["kwargs"] == {
        "local_files_only": True,
        "trust_remote_code": False,
        "use_safetensors": True,
        "token": False,
        "device_map": "cpu",
        "dtype": "float32",
    }
    assert calls["eval"] is True
    assert calls["seed"] == 0
    assert calls["deterministic"] is True
    assert calls["threads"] == 1
    assert versions == dict(CHRONOS_PACKAGE_VERSIONS)


def test_optional_dependency_versions_fail_closed(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(forecast.metadata, "version", lambda _name: "0.0.0")

    with pytest.raises(ValueError, match="versions do not match the lock"):
        forecast._runtime_package_versions(CHRONOS_PACKAGE_VERSIONS)


def test_chronos_rejects_bad_context_quantiles_and_interval() -> None:
    artifact = forecast.ForecastArtifactProvenance(
        model_id=forecast.CHRONOS_MODEL_ID,
        revision=forecast.CHRONOS_MODEL_REVISION,
        license_id="apache-2.0",
        serialization="safetensors",
        config_sha256=forecast.CHRONOS_CONFIG_SHA256,
        weights_sha256=forecast.CHRONOS_WEIGHTS_SHA256,
        bundle_sha256=forecast.CHRONOS_BUNDLE_SHA256,
        runtime_platform=forecast.CHRONOS_RUNTIME_PLATFORM,
        packages=MappingProxyType(dict(CHRONOS_PACKAGE_VERSIONS)),
    )
    backend = forecast.ChronosBoltTinyLocalBackend(
        _pipeline=_FakePipeline(),
        _torch=_FakeTorch(),
        artifact=artifact,
    )
    with pytest.raises(ValueError, match="exactly 64"):
        backend.forecast_one(ForecastRequest(context=(1.0,), interval_z=2.0))
    with pytest.raises(ValueError, match="interval_z=2.0"):
        backend.forecast_one(
            ForecastRequest(context=tuple(float(index) for index in range(64)), interval_z=1.0)
        )

    class BadPipeline(_FakePipeline):
        def predict_quantiles(self, *_args: object, **_kwargs: object) -> object:
            return [[[12.0, 10.0, 8.0]]], [[10.0]]

    bad_backend = forecast.ChronosBoltTinyLocalBackend(
        _pipeline=BadPipeline(),
        _torch=_FakeTorch(),
        artifact=artifact,
    )
    with pytest.raises(ValueError, match="q0.1 <= q0.5 <= q0.9"):
        bad_backend.forecast_one(
            ForecastRequest(context=tuple(float(index) for index in range(64)), interval_z=2.0)
        )


def test_operational_evidence_separates_timing_from_exact_reproducibility() -> None:
    report = _operational_evidence()

    foundation_smoke.validate_operational_evidence(report)

    first, second = report["run_results"]
    assert first["total_replay_runtime_ns"] != second["total_replay_runtime_ns"]
    assert first["peak_rss_kib"] != second["peak_rss_kib"]
    assert report["reproducibility"]["exact_match"] is True


@pytest.mark.parametrize(
    ("mutate", "error"),
    [
        (
            lambda report: report["measurement_method"].__setitem__("peak_rss_unit", "bytes"),
            "measurement method",
        ),
        (
            lambda report: report["run_results"][0].__setitem__("peak_rss_kib", -1),
            "peak RSS",
        ),
        (
            lambda report: report["run_results"][0]["backend_measurements"][0][
                "inference_latency_ns"
            ].pop(),
            "latency sample count",
        ),
        (
            lambda report: report["reproducibility"]["analytical_sha256"].__setitem__(1, "b" * 64),
            "reproducibility",
        ),
        (
            lambda report: report["run_results"][0].__setitem__("analytical_sha256", "A" * 64),
            "analytical_sha256",
        ),
        (
            lambda report: report["run_results"][1].__setitem__("run_id", "run-1"),
            "distinct ordered run IDs",
        ),
        (
            lambda report: report["runtime_attestation"].__setitem__("model_revision", "0" * 40),
            "runtime attestation contract drifted",
        ),
    ],
)
def test_operational_evidence_rejects_method_value_count_and_digest_drift(
    mutate: object,
    error: str,
) -> None:
    report = _operational_evidence()
    mutate(report)  # type: ignore[operator]

    with pytest.raises(ValueError, match=error):
        foundation_smoke.validate_operational_evidence(report)


def test_isolated_environment_routes_every_cache_beneath_tmp(tmp_path: Path) -> None:
    root = tmp_path / "b1"

    env = foundation_smoke._isolated_environment(root)

    for key in (
        "HOME",
        "XDG_CACHE_HOME",
        "PIP_CACHE_DIR",
        "HF_HOME",
        "HF_HUB_CACHE",
        "HUGGINGFACE_HUB_CACHE",
        "HF_XET_CACHE",
        "HF_ASSETS_CACHE",
        "HF_MODULES_CACHE",
        "TRANSFORMERS_CACHE",
        "TORCH_HOME",
        "TORCH_EXTENSIONS_DIR",
        "TORCHINDUCTOR_CACHE_DIR",
        "TRITON_CACHE_DIR",
        "MPLCONFIGDIR",
        "PYTHONPYCACHEPREFIX",
        "TMPDIR",
    ):
        assert Path(env[key]).is_relative_to(root)
    assert env["PYTHONNOUSERSITE"] == "1"
    assert env["PYTHONPATH"] == str((Path.cwd() / "src").resolve())
    assert env["PIP_CONFIG_FILE"] == "/dev/null"
    assert env["HF_HUB_OFFLINE"] == "1"


def test_isolated_runtime_requires_model_and_virtual_environment_under_root(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    root = tmp_path / "b1"
    model = root / "model"
    prefix = root / "replay-venv"
    model.mkdir(parents=True)
    prefix.mkdir()
    monkeypatch.setattr(sys, "prefix", str(prefix))

    foundation_smoke._validate_isolated_runtime_paths(root.resolve(), model)

    monkeypatch.setattr(sys, "prefix", str(tmp_path / "outside-venv"))
    with pytest.raises(ValueError, match="virtual environment"):
        foundation_smoke._validate_isolated_runtime_paths(root.resolve(), model)
    with pytest.raises(ValueError, match="model"):
        foundation_smoke._validate_isolated_runtime_paths(
            root.resolve(), tmp_path / "outside-model"
        )
