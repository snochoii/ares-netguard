#!/usr/bin/env python3
"""Verify one isolated, already-provisioned Chronos replay environment offline."""

from __future__ import annotations

import argparse
import hashlib
import importlib.metadata
import json
import os
import platform
import re
import stat
import subprocess
import sys
import tempfile
from collections.abc import Mapping, Sequence
from pathlib import Path
from typing import Any

SOURCE_IDENTITIES = {
    "requirements-foundation-forecast.in": (
        "29ec8a1dbe075e9d49ff6511fc01a627f21d5fe79f0a461ab44919c30b7cd60d"
    ),
    "requirements-foundation-forecast.lock": (
        "59301d09b3efc73a20466d599d9a56a9634d0c42e001d41947eab2323b677b2e"
    ),
    "pyproject.toml": "c5c0df00a1767ef8b63607831e6e6eee45f293bfc110572f1aa39912f3c701ef",
    "src/ares_netguard/models/manifests/chronos_bolt_tiny_v1.json": (
        "93d6747444e8db71f7aaa9baf601cfdb67f8860b4fb6d953c0203189581dec3a"
    ),
}
LOCKED_REQUIREMENT_RE = re.compile(r"^([A-Za-z0-9_.-]+)==([^ \\\n]+)")
HASH_RE = re.compile(r"--hash=sha256:([0-9a-f]{64})")
EXPECTED_MODEL_FILES = frozenset({"config.json", "model.safetensors"})
EXPECTED_MODEL_FILE_ATTESTATION = [
    {
        "name": "config.json",
        "sha256": "278f0086733031635fb1c861cb01c1bad6477420c7fcb19381a2993e335785e0",
        "size_bytes": 1120,
    },
    {
        "name": "model.safetensors",
        "sha256": "75068728d376d2bec670379eeef4bfb4d24c0cfe24d957451f8d19b447030a32",
        "size_bytes": 34622352,
    },
]
ISOLATION_ROOT_RE = re.compile(r"^/tmp/ares-netguard-b1\.[A-Za-z0-9]+$")
ATTESTATION_FIELDS = frozenset(
    {
        "schema_version",
        "source_identities",
        "python",
        "runtime_platform",
        "package_count",
        "package_set_sha256",
        "wheel_count",
        "wheel_set_sha256",
        "model_id",
        "model_revision",
        "model_files",
        "model_bundle_sha256",
        "pip_check_passed",
    }
)


def _sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _canonical_sha256(value: Any) -> str:
    encoded = json.dumps(value, allow_nan=False, separators=(",", ":"), sort_keys=True).encode(
        "utf-8"
    )
    return hashlib.sha256(encoded).hexdigest()


def _normalize_name(name: str) -> str:
    return re.sub(r"[-_.]+", "-", name).lower()


def _verify_isolated_paths(
    *,
    isolation_root: Path,
    wheelhouse: Path,
    install_report: Path,
    model_root: Path,
    output: Path | None = None,
) -> Path:
    root = isolation_root.resolve(strict=True)
    if not ISOLATION_ROOT_RE.fullmatch(root.as_posix()) or root.parent != Path("/tmp"):
        raise ValueError("B1 isolation root must match /tmp/ares-netguard-b1.XXXXXX")
    for field, path in (
        ("wheelhouse", wheelhouse),
        ("install report", install_report),
        ("model root", model_root),
        ("virtual environment", Path(sys.prefix)),
    ):
        if not path.resolve(strict=True).is_relative_to(root):
            raise ValueError(f"B1 {field} must be under the isolation root")
    if output is not None and not output.resolve(strict=False).is_relative_to(root):
        raise ValueError("B1 attestation output must be under the isolation root")
    return root


def _parse_lock(path: Path) -> dict[str, dict[str, Any]]:
    requirements: dict[str, dict[str, Any]] = {}
    current: str | None = None
    for line in path.read_text(encoding="utf-8").splitlines():
        match = LOCKED_REQUIREMENT_RE.match(line)
        if match:
            current = _normalize_name(match.group(1))
            if current in requirements:
                raise ValueError(f"duplicate locked requirement: {current}")
            requirements[current] = {"version": match.group(2), "hashes": set()}
        for digest in HASH_RE.findall(line):
            if current is None:
                raise ValueError("lock hash appeared before a requirement")
            requirements[current]["hashes"].add(digest)
    if len(requirements) != 41 or any(not item["hashes"] for item in requirements.values()):
        raise ValueError("foundation lock must contain 41 fully hashed requirements")
    return requirements


def _verify_source_identities(repo_root: Path) -> dict[str, str]:
    actual = {path: _sha256_file(repo_root / path) for path in SOURCE_IDENTITIES}
    if actual != SOURCE_IDENTITIES:
        raise ValueError("foundation pin/lock source identity drifted")
    return actual


def _strict_json(path: Path) -> Any:
    def reject(value: str) -> None:
        raise ValueError(f"non-strict JSON constant is not allowed: {value}")

    def reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
        result: dict[str, Any] = {}
        for key, value in pairs:
            if key in result:
                raise ValueError("duplicate JSON object keys are not allowed")
            result[key] = value
        return result

    return json.loads(
        path.read_text(encoding="utf-8"),
        parse_constant=reject,
        object_pairs_hook=reject_duplicate_keys,
    )


def _verify_install_report(
    report_path: Path,
    wheelhouse: Path,
    locked: Mapping[str, Mapping[str, Any]],
) -> tuple[list[dict[str, str]], list[dict[str, str]]]:
    report = _strict_json(report_path)
    if not isinstance(report, Mapping) or not isinstance(report.get("install"), list):
        raise ValueError("pip install report is malformed")
    packages = []
    wheels = []
    wheel_files = sorted(wheelhouse.glob("*.whl"))
    wheel_digests = {_sha256_file(path): path.name for path in wheel_files}
    if len(wheel_files) != len(locked):
        raise ValueError("wheelhouse must contain exactly one wheel per locked requirement")
    for item in report["install"]:
        if not isinstance(item, Mapping):
            raise ValueError("pip install report entry must be an object")
        metadata = item.get("metadata")
        download = item.get("download_info")
        if not isinstance(metadata, Mapping) or not isinstance(download, Mapping):
            raise ValueError("pip install report entry is incomplete")
        name = _normalize_name(str(metadata.get("name")))
        version = str(metadata.get("version"))
        if name not in locked or version != locked[name]["version"]:
            raise ValueError(f"installed package is not pinned: {name}=={version}")
        url = str(download.get("url", ""))
        archive = download.get("archive_info")
        if not url.lower().endswith(".whl") or not isinstance(archive, Mapping):
            raise ValueError(f"installed archive is not a wheel: {name}")
        hashes = archive.get("hashes")
        digest = hashes.get("sha256") if isinstance(hashes, Mapping) else None
        if not isinstance(digest, str) or digest not in locked[name]["hashes"]:
            raise ValueError(f"installed wheel hash is not locked: {name}")
        if digest not in wheel_digests:
            raise ValueError(f"installed wheel is absent from the verified wheelhouse: {name}")
        packages.append({"name": name, "version": version})
        wheels.append({"name": wheel_digests[digest], "sha256": digest})
    packages.sort(key=lambda item: item["name"])
    wheels.sort(key=lambda item: item["name"])
    if len(packages) != len(locked) or {item["name"] for item in packages} != set(locked):
        raise ValueError("pip install report package set does not match the lock")
    return packages, wheels


def _verify_installed_packages(locked: Mapping[str, Mapping[str, Any]]) -> None:
    installed: dict[str, str] = {}
    for distribution in importlib.metadata.distributions():
        name = distribution.metadata.get("Name")
        if name:
            installed[_normalize_name(name)] = distribution.version
    expected = {name: str(item["version"]) for name, item in locked.items()}
    unexpected = set(installed) - set(expected) - {"pip"}
    if unexpected:
        raise ValueError(f"unapproved installed distributions: {sorted(unexpected)}")
    missing_or_drifted = {
        name: (version, installed.get(name))
        for name, version in expected.items()
        if installed.get(name) != version
    }
    if missing_or_drifted:
        raise ValueError(f"installed package versions drifted: {missing_or_drifted}")


def _verify_model(
    manifest_path: Path, model_root: Path
) -> tuple[dict[str, Any], list[dict[str, Any]], str]:
    manifest = _strict_json(manifest_path)
    if not isinstance(manifest, Mapping) or not isinstance(manifest.get("files"), list):
        raise ValueError("Chronos manifest is malformed")
    root_stat = model_root.lstat()
    if stat.S_ISLNK(root_stat.st_mode) or not stat.S_ISDIR(root_stat.st_mode):
        raise ValueError("Chronos model root must be a real directory")
    if root_stat.st_uid != os.getuid() or root_stat.st_mode & (stat.S_IWGRP | stat.S_IWOTH):
        raise ValueError("Chronos model root ownership or mode is unsafe")
    expected = {str(item["name"]): item for item in manifest["files"]}
    actual_names = {path.name for path in model_root.iterdir()}
    if set(expected) != EXPECTED_MODEL_FILES or actual_names != EXPECTED_MODEL_FILES:
        raise ValueError("Chronos model root must contain exactly the pinned files")
    files = []
    for name in sorted(expected):
        path = model_root / name
        path_stat = path.lstat()
        if stat.S_ISLNK(path_stat.st_mode) or not stat.S_ISREG(path_stat.st_mode):
            raise ValueError("Chronos model files must be regular non-symlink files")
        if path_stat.st_uid != os.getuid() or path_stat.st_mode & (stat.S_IWGRP | stat.S_IWOTH):
            raise ValueError("Chronos model file ownership or mode is unsafe")
        item = expected[name]
        digest = _sha256_file(path)
        if path_stat.st_size != item["size_bytes"] or digest != item["sha256"]:
            raise ValueError(f"Chronos model file identity drifted: {name}")
        files.append({"name": name, "size_bytes": path_stat.st_size, "sha256": digest})
    bundle = hashlib.sha256()
    for item in files:
        bundle.update(item["name"].encode("utf-8"))
        bundle.update(b"\0")
        bundle.update(str(item["size_bytes"]).encode("ascii"))
        bundle.update(b"\0")
        bundle.update(item["sha256"].encode("ascii"))
        bundle.update(b"\n")
    return dict(manifest), files, bundle.hexdigest()


def verify_environment(
    *,
    repo_root: Path,
    isolation_root: Path,
    wheelhouse: Path,
    install_report: Path,
    model_root: Path,
) -> dict[str, Any]:
    if (
        sys.version_info[:2] != (3, 12)
        or platform.system() != "Linux"
        or platform.machine() != "x86_64"
    ):
        raise ValueError("foundation replay requires CPython 3.12 on Linux x86_64")
    resolved_isolation_root = _verify_isolated_paths(
        isolation_root=isolation_root,
        wheelhouse=wheelhouse,
        install_report=install_report,
        model_root=model_root,
    )
    identities = _verify_source_identities(repo_root)
    lock_path = repo_root / "requirements-foundation-forecast.lock"
    locked = _parse_lock(lock_path)
    packages, wheels = _verify_install_report(install_report, wheelhouse, locked)
    _verify_installed_packages(locked)
    manifest_path = repo_root / "src/ares_netguard/models/manifests/chronos_bolt_tiny_v1.json"
    manifest, model_files, bundle_sha = _verify_model(manifest_path, model_root)
    if bundle_sha != "9aefb3d869a0a475c81a8685ffaeb99c63a3e3ee1cfc03befeb7cd47b58c00ab":
        raise ValueError("Chronos model bundle digest drifted")
    pip_check_environment = dict(os.environ)
    pip_check_environment.update(
        {
            "PIP_CACHE_DIR": str(resolved_isolation_root / "cache" / "pip"),
            "PIP_CONFIG_FILE": "/dev/null",
            "PIP_DISABLE_PIP_VERSION_CHECK": "1",
            "PYTHONNOUSERSITE": "1",
        }
    )
    pip_check = subprocess.run(
        [sys.executable, "-m", "pip", "check"],
        check=False,
        capture_output=True,
        text=True,
        env=pip_check_environment,
    )
    if pip_check.returncode != 0:
        raise ValueError(f"pip check failed: {pip_check.stdout}{pip_check.stderr}")
    attestation = {
        "schema_version": "foundation_forecast_environment_attestation.v0",
        "source_identities": identities,
        "python": f"{sys.version_info.major}.{sys.version_info.minor}.{sys.version_info.micro}",
        "runtime_platform": "cpython312_linux_x86_64_cpu",
        "package_count": len(packages),
        "package_set_sha256": _canonical_sha256(packages),
        "wheel_count": len(wheels),
        "wheel_set_sha256": _canonical_sha256(wheels),
        "model_id": manifest["model_id"],
        "model_revision": manifest["revision"],
        "model_files": model_files,
        "model_bundle_sha256": bundle_sha,
        "pip_check_passed": True,
    }
    validate_attestation(attestation)
    return attestation


def validate_attestation(attestation: Mapping[str, Any]) -> None:
    if set(attestation) != ATTESTATION_FIELDS:
        raise ValueError("environment attestation fields are invalid")
    if (
        attestation["schema_version"] != "foundation_forecast_environment_attestation.v0"
        or attestation["source_identities"] != SOURCE_IDENTITIES
        or attestation["runtime_platform"] != "cpython312_linux_x86_64_cpu"
        or attestation["package_count"] != 41
        or attestation["package_set_sha256"]
        != "065eb94ed00987015e097f936a19175cab763e99d3d62848c18000eb70fcef5b"
        or attestation["wheel_count"] != 41
        or attestation["wheel_set_sha256"]
        != "deb8a9616a2648c468ae096a867d881f32ec24bba00c0e7323af887895a7bf5b"
        or attestation["model_id"] != "amazon/chronos-bolt-tiny"
        or attestation["model_revision"] != "a0e552de83495b5c28c14c71c374f3e33280b340"
        or attestation["model_files"] != EXPECTED_MODEL_FILE_ATTESTATION
        or attestation["model_bundle_sha256"]
        != "9aefb3d869a0a475c81a8685ffaeb99c63a3e3ee1cfc03befeb7cd47b58c00ab"
        or attestation["pip_check_passed"] is not True
    ):
        raise ValueError("environment attestation contract drifted")
    if not isinstance(attestation["python"], str) or not re.fullmatch(
        r"3\.12\.\d+", attestation["python"]
    ):
        raise ValueError("environment attestation Python version is invalid")


def _atomic_dump(report: Mapping[str, Any], output: Path) -> None:
    if not output.is_absolute() or not output.resolve(strict=False).is_relative_to(Path("/tmp")):
        raise ValueError("environment attestation output must be under /tmp")
    output.parent.mkdir(parents=True, exist_ok=True)
    serialized = json.dumps(report, allow_nan=False, indent=2, sort_keys=True) + "\n"
    with tempfile.NamedTemporaryFile(
        mode="w", encoding="utf-8", dir=output.parent, delete=False
    ) as handle:
        handle.write(serialized)
        temp_path = Path(handle.name)
    os.replace(temp_path, output)


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    parser.add_argument("--isolation-root", type=Path, required=True)
    parser.add_argument("--wheelhouse", type=Path, required=True)
    parser.add_argument("--install-report", type=Path, required=True)
    parser.add_argument("--model-root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args(argv)
    _verify_isolated_paths(
        isolation_root=args.isolation_root,
        wheelhouse=args.wheelhouse,
        install_report=args.install_report,
        model_root=args.model_root,
        output=args.output,
    )
    report = verify_environment(
        repo_root=args.repo_root.resolve(),
        isolation_root=args.isolation_root.resolve(),
        wheelhouse=args.wheelhouse.resolve(),
        install_report=args.install_report.resolve(),
        model_root=args.model_root.resolve(),
    )
    _atomic_dump(report, args.output)
    print("PINNED_ENVIRONMENT: valid")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
