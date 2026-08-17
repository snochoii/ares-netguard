from __future__ import annotations

import hashlib
import json
import secrets
import sys
from pathlib import Path

import pytest

from scripts import verify_foundation_forecast_environment as environment


def test_isolation_contract_rejects_any_provisioned_path_outside_root(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    root = Path("/tmp") / f"ares-netguard-b1.TEST{secrets.token_hex(6)}"
    root.mkdir(mode=0o700)
    try:
        wheelhouse = root / "wheelhouse"
        install_report = root / "install-report.json"
        model = root / "model"
        virtual_environment = root / "replay-venv"
        wheelhouse.mkdir()
        install_report.write_text("{}", encoding="utf-8")
        model.mkdir()
        virtual_environment.mkdir()
        monkeypatch.setattr(sys, "prefix", str(virtual_environment))

        assert (
            environment._verify_isolated_paths(
                isolation_root=root,
                wheelhouse=wheelhouse,
                install_report=install_report,
                model_root=model,
            )
            == root
        )
        with pytest.raises(ValueError, match="wheelhouse"):
            environment._verify_isolated_paths(
                isolation_root=root,
                wheelhouse=tmp_path,
                install_report=install_report,
                model_root=model,
            )
    finally:
        install_report.unlink(missing_ok=True)
        virtual_environment.rmdir()
        model.rmdir()
        wheelhouse.rmdir()
        root.rmdir()


def test_current_pin_sources_and_lock_shape_are_exact() -> None:
    identities = environment._verify_source_identities(Path.cwd())
    locked = environment._parse_lock(Path("requirements-foundation-forecast.lock"))

    assert identities == environment.SOURCE_IDENTITIES
    assert len(locked) == 41
    assert locked["chronos-forecasting"]["version"] == "2.3.1"
    assert locked["torch"]["version"] == "2.12.1+cpu"
    assert all(item["hashes"] for item in locked.values())


def test_install_report_requires_one_locked_wheel_and_matching_hash(tmp_path: Path) -> None:
    wheelhouse = tmp_path / "wheelhouse"
    wheelhouse.mkdir()
    wheel = wheelhouse / "demo-1.0-py3-none-any.whl"
    wheel.write_bytes(b"synthetic wheel stand-in")
    digest = hashlib.sha256(wheel.read_bytes()).hexdigest()
    report = {
        "install": [
            {
                "metadata": {"name": "demo", "version": "1.0"},
                "download_info": {
                    "url": wheel.as_uri(),
                    "archive_info": {"hashes": {"sha256": digest}},
                },
            }
        ]
    }
    report_path = tmp_path / "install-report.json"
    report_path.write_text(json.dumps(report), encoding="utf-8")
    locked = {"demo": {"version": "1.0", "hashes": {digest}}}

    packages, wheels = environment._verify_install_report(report_path, wheelhouse, locked)

    assert packages == [{"name": "demo", "version": "1.0"}]
    assert wheels == [{"name": wheel.name, "sha256": digest}]

    report["install"][0]["download_info"]["archive_info"]["hashes"]["sha256"] = "0" * 64
    report_path.write_text(json.dumps(report), encoding="utf-8")
    with pytest.raises(ValueError, match="hash is not locked"):
        environment._verify_install_report(report_path, wheelhouse, locked)


def test_install_report_and_manifest_parsing_reject_duplicate_keys(tmp_path: Path) -> None:
    install_report = tmp_path / "install-report.json"
    install_report.write_text('{"install":[],"install":[]}', encoding="utf-8")
    with pytest.raises(ValueError, match="duplicate JSON object keys"):
        environment._strict_json(install_report)

    manifest = tmp_path / "manifest.json"
    manifest.write_text('{"revision":"bad","revision":"good"}', encoding="utf-8")
    with pytest.raises(ValueError, match="duplicate JSON object keys"):
        environment._strict_json(manifest)


def test_model_preflight_rejects_extra_files_and_hash_drift(tmp_path: Path) -> None:
    model = tmp_path / "model"
    model.mkdir(mode=0o700)
    config = b"{}"
    weights = b"weights"
    (model / "config.json").write_bytes(config)
    (model / "model.safetensors").write_bytes(weights)
    (model / "config.json").chmod(0o600)
    (model / "model.safetensors").chmod(0o600)
    manifest = {
        "model_id": "amazon/chronos-bolt-tiny",
        "revision": "revision",
        "files": [
            {
                "name": "config.json",
                "size_bytes": len(config),
                "sha256": hashlib.sha256(config).hexdigest(),
            },
            {
                "name": "model.safetensors",
                "size_bytes": len(weights),
                "sha256": hashlib.sha256(weights).hexdigest(),
            },
        ],
    }
    manifest_path = tmp_path / "manifest.json"
    manifest_path.write_text(json.dumps(manifest), encoding="utf-8")

    loaded, files, bundle = environment._verify_model(manifest_path, model)

    assert loaded["revision"] == "revision"
    assert [item["name"] for item in files] == ["config.json", "model.safetensors"]
    assert len(bundle) == 64

    (model / "README.md").write_text("extra", encoding="utf-8")
    with pytest.raises(ValueError, match="exactly the pinned files"):
        environment._verify_model(manifest_path, model)


def test_attestation_rejects_source_or_hash_drift() -> None:
    attestation = {
        "schema_version": "foundation_forecast_environment_attestation.v0",
        "source_identities": dict(environment.SOURCE_IDENTITIES),
        "python": "3.12.3",
        "runtime_platform": "cpython312_linux_x86_64_cpu",
        "package_count": 41,
        "package_set_sha256": "065eb94ed00987015e097f936a19175cab763e99d3d62848c18000eb70fcef5b",
        "wheel_count": 41,
        "wheel_set_sha256": "deb8a9616a2648c468ae096a867d881f32ec24bba00c0e7323af887895a7bf5b",
        "model_id": "amazon/chronos-bolt-tiny",
        "model_revision": "a0e552de83495b5c28c14c71c374f3e33280b340",
        "model_files": list(environment.EXPECTED_MODEL_FILE_ATTESTATION),
        "model_bundle_sha256": "9aefb3d869a0a475c81a8685ffaeb99c63a3e3ee1cfc03befeb7cd47b58c00ab",
        "pip_check_passed": True,
    }
    environment.validate_attestation(attestation)

    attestation["wheel_set_sha256"] = "invalid"
    with pytest.raises(ValueError, match="contract drifted"):
        environment.validate_attestation(attestation)
