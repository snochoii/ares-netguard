"""Offline forecast backends for calibrated time-series residual experiments.

The public request remains intentionally narrow: a backend receives finite
numeric history and an interval width. Entity identifiers, timestamps, paths,
labels, targets, and raw telemetry never cross the inference boundary.
"""

from __future__ import annotations

import hashlib
import json
import math
import os
import platform
import re
import stat
import sys
from collections.abc import Mapping, Sequence
from dataclasses import dataclass, field
from importlib import metadata, resources
from pathlib import Path
from types import MappingProxyType
from typing import Any, ClassVar, Protocol, runtime_checkable

ForecastSetting = str | int | float | bool
MINIMUM_SCALE = 1e-6

DEFAULT_BACKEND_NAME = "rolling_mean_proxy"
CHRONOS_BACKEND_NAME = "chronos_bolt_tiny_local"
SUPPORTED_BACKEND_NAMES = (DEFAULT_BACKEND_NAME, CHRONOS_BACKEND_NAME)

ROLLING_MEAN_PROXY_BACKEND_ID = "rolling_mean_proxy_v1"
ROLLING_MEAN_PROXY_BACKEND_VERSION = "1"
ROLLING_MEAN_PROXY_BACKEND_KIND = "deterministic_proxy"

CHRONOS_BACKEND_ID = "chronos_bolt_tiny_local_v1"
CHRONOS_BACKEND_VERSION = "1"
CHRONOS_BACKEND_KIND = "pretrained_foundation_model"
CHRONOS_MANIFEST_RESOURCE = "chronos_bolt_tiny_v1.json"
CHRONOS_MODEL_ID = "amazon/chronos-bolt-tiny"
CHRONOS_MODEL_REVISION = "a0e552de83495b5c28c14c71c374f3e33280b340"
CHRONOS_CONFIG_SHA256 = "278f0086733031635fb1c861cb01c1bad6477420c7fcb19381a2993e335785e0"
CHRONOS_WEIGHTS_SHA256 = "75068728d376d2bec670379eeef4bfb4d24c0cfe24d957451f8d19b447030a32"
CHRONOS_BUNDLE_SHA256 = "9aefb3d869a0a475c81a8685ffaeb99c63a3e3ee1cfc03befeb7cd47b58c00ab"
CHRONOS_RUNTIME_PLATFORM = "cpython312_linux_x86_64_cpu"
CHRONOS_PACKAGE_VERSIONS: Mapping[str, str] = MappingProxyType(
    {
        "accelerate": "1.14.0",
        "chronos-forecasting": "2.3.1",
        "einops": "0.8.2",
        "huggingface-hub": "1.21.0",
        "numpy": "2.4.6",
        "pandas": "3.0.3",
        "safetensors": "0.8.0",
        "tokenizers": "0.22.2",
        "torch": "2.12.1+cpu",
        "transformers": "5.12.1",
    }
)
CHRONOS_HISTORY_WINDOW = 64
CHRONOS_CALIBRATION_WINDOW = 32
CHRONOS_INTERVAL_Z = 2.0
CHRONOS_PREDICTION_LENGTH = 1
CHRONOS_QUANTILES = (0.1, 0.5, 0.9)
NORMAL_80_INTERVAL_DIVISOR = 2.0 * 1.2815515655446004
CHRONOS_BACKEND_SETTINGS: Mapping[str, ForecastSetting] = MappingProxyType(
    {
        "context_length": CHRONOS_HISTORY_WINDOW,
        "device": "cpu",
        "dtype": "float32",
        "interval_method": "median_plus_minus_interval_z_scale",
        "minimum_scale": MINIMUM_SCALE,
        "point_method": "median_q0_5",
        "prediction_length": CHRONOS_PREDICTION_LENGTH,
        "quantile_levels": "q0_1_q0_5_q0_9",
        "scale_divisor": NORMAL_80_INTERVAL_DIVISOR,
        "scale_method": "normal_equivalent_q0_1_q0_9",
    }
)

MAX_MODEL_FILE_COUNT = 8
MAX_MODEL_BUNDLE_BYTES = 64 * 1024 * 1024
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
SAFE_PACKAGE_VERSION_RE = re.compile(r"^[A-Za-z0-9][A-Za-z0-9.+_-]{0,63}$")


@dataclass(frozen=True, slots=True)
class ForecastRequest:
    """Numeric-only request presented to one forecast backend invocation."""

    context: tuple[float, ...]
    interval_z: float


@dataclass(frozen=True, slots=True)
class ForecastEstimate:
    """One point forecast, interval, and positive standardization scale."""

    mean: float
    lower: float
    upper: float
    scale: float


@dataclass(frozen=True, slots=True)
class ForecastRunRequirements:
    """Exact producer settings validated for a bounded research backend."""

    history_window: int
    calibration_window: int
    interval_z: float


@dataclass(frozen=True, slots=True)
class ForecastBackendSafety:
    """Immutable v1 execution claims for a dependency-free proxy backend."""

    local_only: bool
    synthetic_only: bool
    no_pretrained_model: bool
    no_artifact: bool
    no_network: bool
    no_download: bool
    no_external_service: bool
    no_deployment: bool


@dataclass(frozen=True, slots=True)
class PretrainedForecastBackendSafety:
    """Truthful execution facts for one verified local pretrained artifact."""

    local_only: bool
    synthetic_only: bool
    pretrained_model_used: bool
    operator_provisioned_artifact: bool
    artifact_digest_verified: bool
    local_files_only: bool
    network_used: bool
    download_used: bool
    external_service_used: bool
    remote_code_used: bool
    artifact_persisted_by_ares: bool
    deployment_allowed: bool


@dataclass(frozen=True, slots=True)
class ForecastArtifactProvenance:
    """Sanitized identity for a verified model bundle; never contains a path."""

    model_id: str
    revision: str
    license_id: str
    serialization: str
    config_sha256: str
    weights_sha256: str
    bundle_sha256: str
    runtime_platform: str
    packages: Mapping[str, str] = field(repr=False)


OFFLINE_SYNTHETIC_BACKEND_SAFETY = ForecastBackendSafety(
    local_only=True,
    synthetic_only=True,
    no_pretrained_model=True,
    no_artifact=True,
    no_network=True,
    no_download=True,
    no_external_service=True,
    no_deployment=True,
)

OFFLINE_LOCAL_PRETRAINED_BACKEND_SAFETY = PretrainedForecastBackendSafety(
    local_only=True,
    synthetic_only=True,
    pretrained_model_used=True,
    operator_provisioned_artifact=True,
    artifact_digest_verified=True,
    local_files_only=True,
    network_used=False,
    download_used=False,
    external_service_used=False,
    remote_code_used=False,
    artifact_persisted_by_ares=False,
    deployment_allowed=False,
)


@runtime_checkable
class ForecastBackend(Protocol):
    """Backward-compatible numeric-only backend seam introduced by v1."""

    backend_id: str
    backend_version: str
    backend_kind: str
    safety: ForecastBackendSafety | PretrainedForecastBackendSafety

    @property
    def settings(self) -> Mapping[str, ForecastSetting]: ...

    def forecast_one(self, request: ForecastRequest) -> ForecastEstimate: ...


@runtime_checkable
class PretrainedForecastBackend(ForecastBackend, Protocol):
    """Extended contract for a pinned local pretrained forecast backend."""

    safety: PretrainedForecastBackendSafety
    artifact: ForecastArtifactProvenance
    run_requirements: ForecastRunRequirements


@dataclass(frozen=True, slots=True)
class RollingMeanProxyBackend:
    """Deterministic context mean/population-scale proxy."""

    backend_id: ClassVar[str] = ROLLING_MEAN_PROXY_BACKEND_ID
    backend_version: ClassVar[str] = ROLLING_MEAN_PROXY_BACKEND_VERSION
    backend_kind: ClassVar[str] = ROLLING_MEAN_PROXY_BACKEND_KIND
    safety: ClassVar[ForecastBackendSafety] = OFFLINE_SYNTHETIC_BACKEND_SAFETY
    artifact: ClassVar[None] = None
    run_requirements: ClassVar[None] = None
    _SETTINGS: ClassVar[Mapping[str, ForecastSetting]] = MappingProxyType(
        {
            "mean_method": "context_arithmetic_mean",
            "scale_method": "context_population_standard_deviation",
            "minimum_scale": MINIMUM_SCALE,
        }
    )

    @property
    def settings(self) -> Mapping[str, ForecastSetting]:
        return self._SETTINGS

    def forecast_one(self, request: ForecastRequest) -> ForecastEstimate:
        context, interval_z = validate_forecast_request(request)
        mean = sum(context) / len(context)
        variance = sum((value - mean) ** 2 for value in context) / len(context)
        scale = max(math.sqrt(variance), MINIMUM_SCALE)
        return ForecastEstimate(
            mean=mean,
            lower=mean - interval_z * scale,
            upper=mean + interval_z * scale,
            scale=scale,
        )


@dataclass(frozen=True, slots=True)
class ChronosBoltTinyLocalBackend:
    """Pinned Chronos-Bolt-Tiny pipeline over an operator-provisioned bundle."""

    _pipeline: object = field(repr=False)
    _torch: object = field(repr=False)
    artifact: ForecastArtifactProvenance

    backend_id: ClassVar[str] = CHRONOS_BACKEND_ID
    backend_version: ClassVar[str] = CHRONOS_BACKEND_VERSION
    backend_kind: ClassVar[str] = CHRONOS_BACKEND_KIND
    safety: ClassVar[PretrainedForecastBackendSafety] = OFFLINE_LOCAL_PRETRAINED_BACKEND_SAFETY
    run_requirements: ClassVar[ForecastRunRequirements] = ForecastRunRequirements(
        history_window=CHRONOS_HISTORY_WINDOW,
        calibration_window=CHRONOS_CALIBRATION_WINDOW,
        interval_z=CHRONOS_INTERVAL_Z,
    )
    _SETTINGS: ClassVar[Mapping[str, ForecastSetting]] = CHRONOS_BACKEND_SETTINGS

    @property
    def settings(self) -> Mapping[str, ForecastSetting]:
        return self._SETTINGS

    def forecast_one(self, request: ForecastRequest) -> ForecastEstimate:
        context, interval_z = validate_forecast_request(request)
        if len(context) != CHRONOS_HISTORY_WINDOW:
            raise ValueError(
                f"chronos backend requires exactly {CHRONOS_HISTORY_WINDOW} context values"
            )
        if not math.isclose(interval_z, CHRONOS_INTERVAL_Z, rel_tol=0.0, abs_tol=1e-12):
            raise ValueError(f"chronos backend requires interval_z={CHRONOS_INTERVAL_Z}")

        try:
            tensor = self._torch.tensor(context, dtype=self._torch.float32)
            with self._torch.inference_mode():
                result = self._pipeline.predict_quantiles(
                    tensor,
                    prediction_length=CHRONOS_PREDICTION_LENGTH,
                    quantile_levels=list(CHRONOS_QUANTILES),
                )
        except Exception:
            raise ValueError("chronos backend inference failed") from None

        if not isinstance(result, tuple) or len(result) != 2:
            raise ValueError("chronos backend returned malformed quantile output")
        q10, q50, q90 = _extract_single_step_quantiles(result[0])
        if not q10 <= q50 <= q90:
            raise ValueError("chronos quantiles require q0.1 <= q0.5 <= q0.9")
        scale = max((q90 - q10) / NORMAL_80_INTERVAL_DIVISOR, MINIMUM_SCALE)
        return validate_forecast_estimate(
            ForecastEstimate(
                mean=q50,
                lower=q50 - interval_z * scale,
                upper=q50 + interval_z * scale,
                scale=scale,
            )
        )


@dataclass(frozen=True, slots=True)
class _ManifestFile:
    name: str
    size_bytes: int
    sha256: str


@dataclass(frozen=True, slots=True)
class _ChronosManifest:
    model_id: str
    revision: str
    license_id: str
    serialization: str
    runtime_platform: str
    packages: Mapping[str, str]
    files: tuple[_ManifestFile, ...]


@dataclass(frozen=True, slots=True)
class _VerifiedBundle:
    root: Path = field(repr=False)
    manifest: _ChronosManifest
    bundle_sha256: str


def resolve_forecast_backend(
    name: str,
    *,
    model_root: str | Path | None = None,
) -> ForecastBackend:
    """Resolve one backend through a closed, no-fallback allowlist."""
    if name == DEFAULT_BACKEND_NAME:
        if model_root is not None:
            raise ValueError("rolling_mean_proxy does not accept a model root")
        return RollingMeanProxyBackend()
    if name == CHRONOS_BACKEND_NAME:
        if model_root is None:
            raise ValueError("chronos_bolt_tiny_local requires an explicit model root")
        return _build_chronos_backend(model_root)
    allowed = ", ".join(repr(value) for value in SUPPORTED_BACKEND_NAMES)
    raise ValueError(f"unsupported forecast backend {name!r}; allowed backends: {allowed}")


def validate_forecast_run_requirements(
    backend: ForecastBackend,
    *,
    history_window: int,
    calibration_window: int,
    interval_z: float,
) -> None:
    """Fail before prediction if a bounded backend is used outside its validated run."""
    requirements = getattr(backend, "run_requirements", None)
    if requirements is None:
        return
    if not isinstance(requirements, ForecastRunRequirements):
        raise ValueError("forecast backend run_requirements are malformed")
    actual = (history_window, calibration_window, float(interval_z))
    expected = (
        requirements.history_window,
        requirements.calibration_window,
        requirements.interval_z,
    )
    if actual != expected:
        raise ValueError(
            "forecast backend requires "
            f"history_window={expected[0]}, calibration_window={expected[1]}, "
            f"interval_z={expected[2]}"
        )


def validate_forecast_request(request: ForecastRequest) -> tuple[tuple[float, ...], float]:
    """Validate and return a forecast request without widening its data surface."""
    if not isinstance(request, ForecastRequest):
        raise ValueError("forecast request must be a ForecastRequest")
    if not isinstance(request.context, tuple) or not request.context:
        raise ValueError("forecast request context must be a non-empty tuple")
    context = tuple(
        _finite_number(value, f"forecast request context[{index}]")
        for index, value in enumerate(request.context)
    )
    interval_z = _finite_number(request.interval_z, "forecast request interval_z")
    if interval_z <= 0.0:
        raise ValueError("forecast request interval_z must be positive")
    return context, interval_z


def validate_forecast_estimate(estimate: ForecastEstimate) -> ForecastEstimate:
    """Reject malformed/non-finite backend output before residual scoring."""
    if not isinstance(estimate, ForecastEstimate):
        raise ValueError("forecast backend must return a ForecastEstimate")
    mean = _finite_number(estimate.mean, "forecast estimate mean")
    lower = _finite_number(estimate.lower, "forecast estimate lower")
    upper = _finite_number(estimate.upper, "forecast estimate upper")
    scale = _finite_number(estimate.scale, "forecast estimate scale")
    if scale <= 0.0:
        raise ValueError("forecast estimate scale must be positive")
    if not lower <= mean <= upper:
        raise ValueError("forecast estimate requires lower <= mean <= upper")
    return ForecastEstimate(mean=mean, lower=lower, upper=upper, scale=scale)


def _build_chronos_backend(model_root: str | Path) -> ChronosBoltTinyLocalBackend:
    manifest = _load_chronos_manifest()
    bundle = _verify_model_bundle(model_root, manifest)
    pipeline, torch_module, package_versions = _load_chronos_runtime(bundle)
    post_load_bundle = _verify_model_bundle(model_root, manifest)
    if (
        post_load_bundle.root != bundle.root
        or post_load_bundle.bundle_sha256 != bundle.bundle_sha256
    ):
        raise ValueError("chronos model bundle changed while it was being loaded")
    artifact = ForecastArtifactProvenance(
        model_id=manifest.model_id,
        revision=manifest.revision,
        license_id=manifest.license_id,
        serialization=manifest.serialization,
        config_sha256=_manifest_file(manifest, "config.json").sha256,
        weights_sha256=_manifest_file(manifest, "model.safetensors").sha256,
        bundle_sha256=bundle.bundle_sha256,
        runtime_platform=manifest.runtime_platform,
        packages=MappingProxyType(dict(sorted(package_versions.items()))),
    )
    return ChronosBoltTinyLocalBackend(
        _pipeline=pipeline,
        _torch=torch_module,
        artifact=artifact,
    )


def _load_chronos_manifest() -> _ChronosManifest:
    raw_text = (
        resources.files("ares_netguard.models.manifests")
        .joinpath(CHRONOS_MANIFEST_RESOURCE)
        .read_text(encoding="utf-8")
    )
    raw = _loads_strict(raw_text, "chronos artifact manifest")
    if not isinstance(raw, Mapping):
        raise ValueError("chronos artifact manifest must be an object")
    _exact_fields(
        raw,
        {
            "schema_version",
            "model_id",
            "revision",
            "license_id",
            "serialization",
            "runtime_platform",
            "packages",
            "files",
        },
        "chronos artifact manifest",
    )
    if raw["schema_version"] != "forecast_artifact_manifest.v0":
        raise ValueError("chronos artifact manifest schema is unsupported")
    if raw["model_id"] != CHRONOS_MODEL_ID:
        raise ValueError("chronos artifact manifest model_id is unsupported")
    if raw["revision"] != CHRONOS_MODEL_REVISION:
        raise ValueError("chronos artifact manifest revision is unsupported")
    if raw["license_id"] != "apache-2.0" or raw["serialization"] != "safetensors":
        raise ValueError("chronos artifact manifest license or serialization is unsupported")
    if raw["runtime_platform"] != CHRONOS_RUNTIME_PLATFORM:
        raise ValueError("chronos artifact manifest runtime platform is unsupported")

    packages = raw["packages"]
    if not isinstance(packages, Mapping):
        raise ValueError("chronos artifact manifest packages must be an object")
    _exact_fields(
        packages,
        set(CHRONOS_PACKAGE_VERSIONS),
        "chronos artifact manifest packages",
    )
    normalized_packages: dict[str, str] = {}
    for name, version in sorted(packages.items()):
        if not isinstance(version, str) or not SAFE_PACKAGE_VERSION_RE.fullmatch(version):
            raise ValueError("chronos artifact manifest contains an unsafe package version")
        normalized_packages[str(name)] = version
    if normalized_packages != dict(CHRONOS_PACKAGE_VERSIONS):
        raise ValueError("chronos artifact manifest package versions are not pinned")

    files = raw["files"]
    if not isinstance(files, list) or not 1 <= len(files) <= MAX_MODEL_FILE_COUNT:
        raise ValueError("chronos artifact manifest files must be a bounded list")
    parsed_files: list[_ManifestFile] = []
    for raw_file in files:
        if not isinstance(raw_file, Mapping):
            raise ValueError("chronos artifact manifest file entries must be objects")
        _exact_fields(raw_file, {"name", "size_bytes", "sha256"}, "manifest file")
        name = raw_file["name"]
        size_bytes = raw_file["size_bytes"]
        digest = raw_file["sha256"]
        if name not in {"config.json", "model.safetensors"}:
            raise ValueError("chronos artifact manifest contains an unsupported file")
        if isinstance(size_bytes, bool) or not isinstance(size_bytes, int) or size_bytes <= 0:
            raise ValueError("chronos artifact manifest file size must be positive")
        if not isinstance(digest, str) or not SHA256_RE.fullmatch(digest):
            raise ValueError("chronos artifact manifest file digest must be SHA-256")
        parsed_files.append(_ManifestFile(name=name, size_bytes=size_bytes, sha256=digest))
    if {item.name for item in parsed_files} != {"config.json", "model.safetensors"}:
        raise ValueError("chronos artifact manifest requires config and safetensors files")
    if len({item.name for item in parsed_files}) != len(parsed_files):
        raise ValueError("chronos artifact manifest file names must be unique")
    if sum(item.size_bytes for item in parsed_files) > MAX_MODEL_BUNDLE_BYTES:
        raise ValueError("chronos artifact manifest bundle exceeds the size limit")

    return _ChronosManifest(
        model_id=str(raw["model_id"]),
        revision=str(raw["revision"]),
        license_id=str(raw["license_id"]),
        serialization=str(raw["serialization"]),
        runtime_platform=str(raw["runtime_platform"]),
        packages=MappingProxyType(normalized_packages),
        files=tuple(sorted(parsed_files, key=lambda item: item.name)),
    )


def _verify_model_bundle(model_root: str | Path, manifest: _ChronosManifest) -> _VerifiedBundle:
    root = Path(model_root)
    if not root.is_absolute() or ".." in root.parts:
        raise ValueError("chronos model root must be an absolute normalized path")
    try:
        root_stat = root.lstat()
    except OSError:
        raise ValueError("chronos model root is unavailable") from None
    if stat.S_ISLNK(root_stat.st_mode) or not stat.S_ISDIR(root_stat.st_mode):
        raise ValueError("chronos model root must be a non-symlink directory")
    _validate_bundle_path_ownership(root_stat, "chronos model root")
    resolved = root.resolve(strict=True)
    if not _is_allowed_model_root(root, resolved):
        raise ValueError(
            "chronos model root must be under /tmp, data/models, .runtime/models, "
            "or artifacts/models"
        )

    try:
        children = list(root.iterdir())
    except OSError:
        raise ValueError("chronos model bundle cannot be inspected") from None
    expected_names = {item.name for item in manifest.files}
    if {child.name for child in children} != expected_names or len(children) != len(expected_names):
        raise ValueError("chronos model bundle must contain exactly the pinned files")

    for manifest_file in manifest.files:
        candidate = root / manifest_file.name
        try:
            candidate_stat = candidate.lstat()
        except OSError:
            raise ValueError("chronos model bundle is missing a pinned file") from None
        if stat.S_ISLNK(candidate_stat.st_mode) or not stat.S_ISREG(candidate_stat.st_mode):
            raise ValueError("chronos model bundle files must be regular non-symlink files")
        _validate_bundle_path_ownership(candidate_stat, "chronos model bundle file")
        if candidate.resolve(strict=True).parent != resolved:
            raise ValueError("chronos model bundle file escapes its root")
        if candidate_stat.st_size != manifest_file.size_bytes:
            raise ValueError("chronos model bundle file size does not match the manifest")
        if _sha256_file(candidate) != manifest_file.sha256:
            raise ValueError("chronos model bundle file digest does not match the manifest")

    _validate_pinned_chronos_config(root / "config.json")
    bundle_sha256 = _bundle_digest(manifest.files)
    return _VerifiedBundle(
        root=resolved,
        manifest=manifest,
        bundle_sha256=bundle_sha256,
    )


def _validate_pinned_chronos_config(path: Path) -> None:
    try:
        raw = _loads_strict(path.read_text(encoding="utf-8"), "chronos config")
    except OSError:
        raise ValueError("chronos config cannot be read") from None
    if not isinstance(raw, Mapping):
        raise ValueError("chronos config must be an object")
    if raw.get("architectures") != ["ChronosBoltModelForForecasting"]:
        raise ValueError("chronos config architecture is unsupported")
    if raw.get("chronos_pipeline_class") != "ChronosBoltPipeline":
        raise ValueError("chronos config pipeline class is unsupported")
    if raw.get("auto_map") is not None:
        raise ValueError("chronos config remote-code mapping is forbidden")
    chronos_config = raw.get("chronos_config")
    if not isinstance(chronos_config, Mapping):
        raise ValueError("chronos config requires chronos_config")
    if chronos_config.get("input_patch_size") != 16:
        raise ValueError("chronos config input patch size is unsupported")
    if chronos_config.get("prediction_length") != 64:
        raise ValueError("chronos config prediction length is unsupported")
    if chronos_config.get("quantiles") != [index / 10 for index in range(1, 10)]:
        raise ValueError("chronos config quantiles are unsupported")


def _load_chronos_runtime(
    bundle: _VerifiedBundle,
) -> tuple[object, object, dict[str, str]]:
    _validate_runtime_platform(bundle.manifest.runtime_platform)
    versions = _runtime_package_versions(bundle.manifest.packages)
    os.environ["HF_HUB_OFFLINE"] = "1"
    os.environ["TRANSFORMERS_OFFLINE"] = "1"
    os.environ["HF_HUB_DISABLE_TELEMETRY"] = "1"
    os.environ["DO_NOT_TRACK"] = "1"
    try:
        import torch
        from chronos import BaseChronosPipeline

        torch.manual_seed(0)
        torch.use_deterministic_algorithms(True)
        torch.set_num_threads(1)
        pipeline = BaseChronosPipeline.from_pretrained(
            bundle.root,
            local_files_only=True,
            trust_remote_code=False,
            use_safetensors=True,
            token=False,
            device_map="cpu",
            dtype=torch.float32,
        )
        model = getattr(pipeline, "model", None)
        if model is None or not callable(getattr(model, "eval", None)):
            raise ValueError("loaded pipeline has no evaluable model")
        model.eval()
    except Exception:
        raise ValueError(
            "chronos backend initialization failed after artifact verification"
        ) from None
    return pipeline, torch, versions


def _runtime_package_versions(expected: Mapping[str, str]) -> dict[str, str]:
    actual: dict[str, str] = {}
    for name, expected_version in sorted(expected.items()):
        try:
            installed = metadata.version(name)
        except metadata.PackageNotFoundError:
            raise ValueError("chronos backend optional dependencies are not installed") from None
        if installed != expected_version:
            raise ValueError("chronos backend optional dependency versions do not match the lock")
        actual[name] = installed
    return actual


def _validate_runtime_platform(expected: str) -> None:
    actual = (
        f"{platform.python_implementation().lower()}"
        f"{sys.version_info.major}{sys.version_info.minor}_"
        f"{platform.system().lower()}_{platform.machine().lower()}_cpu"
    )
    if actual != expected:
        raise ValueError("chronos backend requires the pinned CPython 3.12 Linux x86-64 runtime")


def _extract_single_step_quantiles(raw_quantiles: object) -> tuple[float, float, float]:
    value = raw_quantiles
    for method_name in ("detach", "cpu"):
        method = getattr(value, method_name, None)
        if callable(method):
            value = method()
    tolist = getattr(value, "tolist", None)
    if callable(tolist):
        value = tolist()
    if not isinstance(value, Sequence) or isinstance(value, str | bytes | bytearray):
        raise ValueError("chronos backend returned malformed quantile output")
    if len(value) != 1 or not _is_sequence(value[0]) or len(value[0]) != 1:
        raise ValueError("chronos backend returned malformed quantile output")
    horizon = value[0][0]
    if not _is_sequence(horizon) or len(horizon) != 3:
        raise ValueError("chronos backend returned malformed quantile output")
    return tuple(
        _finite_number(item, f"chronos quantile[{index}]") for index, item in enumerate(horizon)
    )  # type: ignore[return-value]


def _is_sequence(value: object) -> bool:
    return isinstance(value, Sequence) and not isinstance(value, str | bytes | bytearray)


def _is_allowed_model_root(lexical: Path, resolved: Path) -> bool:
    repo = _repository_root()
    candidates = (
        Path("/tmp").resolve(),
        repo / "data" / "models",
        repo / ".runtime" / "models",
        repo / "artifacts" / "models",
    )
    for candidate in candidates:
        allowed = candidate.resolve(strict=False)
        if _is_relative_to(lexical, candidate) and _is_relative_to(resolved, allowed):
            return True
    return False


def _validate_bundle_path_ownership(path_stat: os.stat_result, field: str) -> None:
    if path_stat.st_uid != os.geteuid():
        raise ValueError(f"{field} must be owned by the current user")
    if path_stat.st_mode & (stat.S_IWGRP | stat.S_IWOTH):
        raise ValueError(f"{field} must not be group- or other-writable")


def _manifest_file(manifest: _ChronosManifest, name: str) -> _ManifestFile:
    for item in manifest.files:
        if item.name == name:
            return item
    raise ValueError("chronos artifact manifest is missing a required file")


def _bundle_digest(files: Sequence[_ManifestFile]) -> str:
    digest = hashlib.sha256()
    for item in sorted(files, key=lambda value: value.name):
        digest.update(item.name.encode("utf-8"))
        digest.update(b"\0")
        digest.update(str(item.size_bytes).encode("ascii"))
        digest.update(b"\0")
        digest.update(item.sha256.encode("ascii"))
        digest.update(b"\n")
    return digest.hexdigest()


def _sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    try:
        with path.open("rb") as handle:
            while chunk := handle.read(1024 * 1024):
                digest.update(chunk)
    except OSError:
        raise ValueError("chronos model bundle file cannot be read") from None
    return digest.hexdigest()


def _loads_strict(text: str, field: str) -> Any:
    def reject_constant(value: str) -> None:
        raise ValueError(f"{field} contains non-strict JSON constant {value}")

    def reject_duplicates(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
        result: dict[str, Any] = {}
        for key, value in pairs:
            if key in result:
                raise ValueError(f"{field} contains duplicate key {key!r}")
            result[key] = value
        return result

    try:
        return json.loads(
            text,
            parse_constant=reject_constant,
            object_pairs_hook=reject_duplicates,
        )
    except json.JSONDecodeError as exc:
        raise ValueError(f"{field} is invalid JSON") from exc


def _exact_fields(raw: Mapping[str, Any], expected: set[str], field: str) -> None:
    actual = set(raw)
    if actual != expected:
        missing = sorted(expected - actual)
        unknown = sorted(actual - expected)
        raise ValueError(f"{field} fields are invalid; missing={missing}, unknown={unknown}")


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


def _finite_number(raw_value: object, field: str) -> float:
    if isinstance(raw_value, bool) or not isinstance(raw_value, int | float):
        raise ValueError(f"{field} must be a finite number")
    value = float(raw_value)
    if not math.isfinite(value):
        raise ValueError(f"{field} must be a finite number")
    return value
