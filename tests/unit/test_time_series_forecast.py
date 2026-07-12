from __future__ import annotations

import importlib
import socket
import subprocess
from dataclasses import FrozenInstanceError

import pytest

from ares_netguard.models.time_series_forecast import (
    DEFAULT_BACKEND_NAME,
    OFFLINE_SYNTHETIC_BACKEND_SAFETY,
    ForecastBackend,
    ForecastEstimate,
    ForecastRequest,
    RollingMeanProxyBackend,
    resolve_forecast_backend,
    validate_forecast_estimate,
)


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
