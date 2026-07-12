"""Offline forecast backend contract for time-series residual experiments.

The seam is intentionally narrow: a backend receives only finite numeric history
and an interval width. It never receives entity identifiers, timestamps, paths,
raw telemetry payloads, or the target being scored.
"""

from __future__ import annotations

import math
from collections.abc import Mapping
from dataclasses import dataclass
from types import MappingProxyType
from typing import ClassVar, Protocol, runtime_checkable

DEFAULT_BACKEND_NAME = "rolling_mean_proxy"
ROLLING_MEAN_PROXY_BACKEND_ID = "rolling_mean_proxy_v1"
ROLLING_MEAN_PROXY_BACKEND_VERSION = "1"
ROLLING_MEAN_PROXY_BACKEND_KIND = "deterministic_proxy"
MINIMUM_SCALE = 1e-6

ForecastSetting = str | int | float | bool


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
class ForecastBackendSafety:
    """Immutable execution/scope claims required from a v1 backend."""

    local_only: bool
    synthetic_only: bool
    no_pretrained_model: bool
    no_artifact: bool
    no_network: bool
    no_download: bool
    no_external_service: bool
    no_deployment: bool


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


@runtime_checkable
class ForecastBackend(Protocol):
    """Fixed-identity, offline forecast backend used by the research adapter."""

    backend_id: str
    backend_version: str
    backend_kind: str
    safety: ForecastBackendSafety

    @property
    def settings(self) -> Mapping[str, ForecastSetting]: ...

    def forecast_one(self, request: ForecastRequest) -> ForecastEstimate: ...


@dataclass(frozen=True, slots=True)
class RollingMeanProxyBackend:
    """Deterministic context mean/population-scale proxy.

    This is not a pretrained model. The class has no configurable constructor
    fields, and its settings mapping is read-only.
    """

    backend_id: ClassVar[str] = ROLLING_MEAN_PROXY_BACKEND_ID
    backend_version: ClassVar[str] = ROLLING_MEAN_PROXY_BACKEND_VERSION
    backend_kind: ClassVar[str] = ROLLING_MEAN_PROXY_BACKEND_KIND
    safety: ClassVar[ForecastBackendSafety] = OFFLINE_SYNTHETIC_BACKEND_SAFETY
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


def resolve_forecast_backend(name: str) -> ForecastBackend:
    """Resolve one CLI backend name through a closed, no-fallback allowlist."""
    if name == DEFAULT_BACKEND_NAME:
        return RollingMeanProxyBackend()
    raise ValueError(
        f"unsupported forecast backend {name!r}; allowed backend: {DEFAULT_BACKEND_NAME!r}"
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


def _finite_number(raw_value: object, field: str) -> float:
    if isinstance(raw_value, bool) or not isinstance(raw_value, int | float):
        raise ValueError(f"{field} must be a finite number")
    value = float(raw_value)
    if not math.isfinite(value):
        raise ValueError(f"{field} must be a finite number")
    return value
