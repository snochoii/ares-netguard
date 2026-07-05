"""Local model registry metadata contract.

The v0 metadata derives only from a validated synthetic model evaluation
bundle. It does not persist registry state, promote models, load model
artifacts, execute inference, inspect telemetry, deploy rules, or call external
services.
"""

from __future__ import annotations

import argparse
import json
from collections.abc import Mapping, Sequence
from pathlib import Path
from typing import Any

from ares_netguard.models import evaluation_bundle

REPORT_SCHEMA_VERSION = "model_registry_metadata.v0"
METADATA_SCOPE = "local_synthetic_model_registry_metadata"
REGISTRY_STATE = "observed_synthetic_only"
PROMOTION_STATE = "not_promoted"

METADATA_FIELDS = frozenset(
    {
        "schema_version",
        "metadata_scope",
        "source_bundle_schema",
        "entries",
        "aggregate_summary",
        "safety_flags",
        "non_claims",
    }
)
ENTRY_FIELDS = frozenset(
    {
        "model_id",
        "registry_state",
        "promotion_state",
        "observed_source_schemas",
        "observed_source_names",
        "source_count",
        "has_score_rows",
        "human_review_required",
        "deployment_allowed",
    }
)
AGGREGATE_SUMMARY_FIELDS = frozenset(
    {
        "model_count",
        "schemas_present",
        "models_with_score_rows",
        "deployment_allowed",
    }
)
SAFETY_FLAG_FIELDS = frozenset(
    {
        "local_only",
        "strict_json_loaded",
        "derived_from_evaluation_bundle_only",
        "input_paths_copied",
        "source_filenames_copied",
        "raw_identifiers_copied",
        "generated_artifact_references_copied",
        "secrets_detected",
        "report_payload_copied",
        "live_capture_used",
        "external_services_used",
        "deployment_allowed",
    }
)

JsonMap = dict[str, Any]


def load_evaluation_bundle(path: str | Path) -> JsonMap:
    """Load and validate one strict ``model_evaluation_bundle.v0`` JSON file."""
    source = Path(path)
    if source.is_dir():
        raise ValueError(f"evaluation bundle path must be a file, not a directory: {source}")
    if not source.exists():
        raise ValueError(f"evaluation bundle path does not exist: {source}")

    payload = evaluation_bundle._loads_strict(source.read_text(encoding="utf-8"))
    if not isinstance(payload, Mapping):
        raise ValueError("evaluation bundle must be a JSON object")
    loaded = dict(payload)
    evaluation_bundle.validate_evaluation_bundle(loaded)
    return loaded


def generate_registry_metadata(bundle: Mapping[str, Any]) -> JsonMap:
    """Generate deterministic synthetic-only registry metadata from a bundle."""
    evaluation_bundle.validate_evaluation_bundle(bundle)

    entries = _entries_from_bundle(bundle)
    metadata = {
        "schema_version": REPORT_SCHEMA_VERSION,
        "metadata_scope": METADATA_SCOPE,
        "source_bundle_schema": evaluation_bundle.REPORT_SCHEMA_VERSION,
        "entries": entries,
        "aggregate_summary": _aggregate_entries(entries),
        "safety_flags": {
            "local_only": True,
            "strict_json_loaded": True,
            "derived_from_evaluation_bundle_only": True,
            "input_paths_copied": False,
            "source_filenames_copied": False,
            "raw_identifiers_copied": False,
            "generated_artifact_references_copied": False,
            "secrets_detected": False,
            "report_payload_copied": False,
            "live_capture_used": False,
            "external_services_used": False,
            "deployment_allowed": False,
        },
        "non_claims": [
            "not_persistent_model_registry",
            "not_model_promotion_gate",
            "not_deployment_approval",
            "not_live_capture",
            "not_external_enrichment",
            "not_rule_deployment",
            "not_native_runtime_execution",
        ],
    }
    validate_registry_metadata(metadata)
    return metadata


def dump_metadata(
    metadata: Mapping[str, Any],
    path: str | Path,
    *,
    repo_root: str | Path | None = None,
) -> None:
    """Write validated registry metadata to a non-committed output location."""
    output = evaluation_bundle._validated_output_path(path, repo_root=repo_root)
    validate_registry_metadata(metadata)
    output.write_text(
        json.dumps(metadata, allow_nan=False, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def validate_registry_metadata(metadata: Mapping[str, Any]) -> None:
    """Validate the strict ``model_registry_metadata.v0`` output contract."""
    if not isinstance(metadata, Mapping):
        raise ValueError("registry metadata must be an object")
    evaluation_bundle._require_exact_fields(metadata, METADATA_FIELDS, "registry metadata")
    evaluation_bundle._validate_safe_tree(metadata, "registry metadata")

    if metadata["schema_version"] != REPORT_SCHEMA_VERSION:
        raise ValueError(f"registry metadata requires schema_version '{REPORT_SCHEMA_VERSION}'")
    if metadata["metadata_scope"] != METADATA_SCOPE:
        raise ValueError(f"metadata_scope must be {METADATA_SCOPE}")
    if metadata["source_bundle_schema"] != evaluation_bundle.REPORT_SCHEMA_VERSION:
        raise ValueError("source_bundle_schema must be model_evaluation_bundle.v0")

    entries = evaluation_bundle._bounded_list(metadata["entries"], "entries")
    for entry in entries:
        _validate_registry_entry(entry)
    model_ids = [str(entry["model_id"]) for entry in entries]
    if model_ids != sorted(set(model_ids)):
        raise ValueError("entries must be sorted with one entry per model_id")

    _validate_aggregate_summary(metadata["aggregate_summary"])
    if metadata["aggregate_summary"] != _aggregate_entries(entries):
        raise ValueError("aggregate_summary must be derived from entries")

    _validate_safety_flags(metadata["safety_flags"])
    expected_non_claims = [
        "not_persistent_model_registry",
        "not_model_promotion_gate",
        "not_deployment_approval",
        "not_live_capture",
        "not_external_enrichment",
        "not_rule_deployment",
        "not_native_runtime_execution",
    ]
    if evaluation_bundle._bounded_list(metadata["non_claims"], "non_claims") != expected_non_claims:
        raise ValueError("non_claims must match the v0 non-claim list")


def _entries_from_bundle(bundle: Mapping[str, Any]) -> list[JsonMap]:
    model_sources: dict[str, list[Mapping[str, Any]]] = {}
    for summary in evaluation_bundle._bounded_list(bundle["source_summaries"], "source_summaries"):
        if not isinstance(summary, Mapping):
            raise ValueError("source summary must be an object")
        for model_id in evaluation_bundle._bounded_list(summary["model_ids"], "model_ids"):
            model_sources.setdefault(str(model_id), []).append(summary)

    entries: list[JsonMap] = []
    for model_id in sorted(model_sources):
        summaries = model_sources[model_id]
        source_schemas = sorted({str(summary["source_schema"]) for summary in summaries})
        source_names = sorted(str(summary["source_name"]) for summary in summaries)
        entries.append(
            {
                "model_id": model_id,
                "registry_state": REGISTRY_STATE,
                "promotion_state": PROMOTION_STATE,
                "observed_source_schemas": source_schemas,
                "observed_source_names": source_names,
                "source_count": len(summaries),
                "has_score_rows": any(summary["score_row_count"] > 0 for summary in summaries),
                "human_review_required": True,
                "deployment_allowed": False,
            }
        )
    return entries


def _aggregate_entries(entries: Sequence[Mapping[str, Any]]) -> JsonMap:
    schemas: set[str] = set()
    models_with_score_rows: list[str] = []
    for entry in entries:
        schemas.update(str(schema) for schema in entry["observed_source_schemas"])
        if entry["has_score_rows"]:
            models_with_score_rows.append(str(entry["model_id"]))

    return {
        "model_count": len(entries),
        "schemas_present": sorted(schemas),
        "models_with_score_rows": sorted(models_with_score_rows),
        "deployment_allowed": False,
    }


def _validate_registry_entry(entry: Any) -> None:
    if not isinstance(entry, Mapping):
        raise ValueError("registry entry must be an object")
    evaluation_bundle._require_exact_fields(entry, ENTRY_FIELDS, "registry entry")

    evaluation_bundle._required_model_id(entry["model_id"], "model_id")
    if entry["registry_state"] != REGISTRY_STATE:
        raise ValueError(f"registry_state must be {REGISTRY_STATE}")
    if entry["promotion_state"] != PROMOTION_STATE:
        raise ValueError(f"promotion_state must be {PROMOTION_STATE}")

    source_schemas = evaluation_bundle._bounded_list(
        entry["observed_source_schemas"], "observed_source_schemas"
    )
    if not source_schemas:
        raise ValueError("observed_source_schemas must not be empty")
    if source_schemas != sorted(set(source_schemas)):
        raise ValueError("observed_source_schemas must be sorted and unique")
    for schema in source_schemas:
        if schema not in evaluation_bundle.SUPPORTED_SOURCE_SCHEMAS:
            raise ValueError("observed_source_schemas contains unsupported schema")

    source_names = evaluation_bundle._bounded_list(
        entry["observed_source_names"], "observed_source_names"
    )
    if not source_names:
        raise ValueError("observed_source_names must not be empty")
    if source_names != sorted(source_names):
        raise ValueError("observed_source_names must be sorted")
    for source_name in source_names:
        text = evaluation_bundle._required_text(source_name, "observed_source_names")
        if not evaluation_bundle.SAFE_SOURCE_NAME_RE.fullmatch(text):
            raise ValueError("observed_source_names must be generated source names")

    source_count = evaluation_bundle._positive_int(entry["source_count"], "source_count")
    if source_count != len(source_names):
        raise ValueError("source_count must match observed_source_names")
    if not isinstance(entry["has_score_rows"], bool):
        raise ValueError("has_score_rows must be a boolean")
    if entry["human_review_required"] is not True:
        raise ValueError("human_review_required must be true")
    if entry["deployment_allowed"] is not False:
        raise ValueError("deployment_allowed must be false")


def _validate_aggregate_summary(summary: Any) -> None:
    if not isinstance(summary, Mapping):
        raise ValueError("aggregate_summary must be an object")
    evaluation_bundle._require_exact_fields(summary, AGGREGATE_SUMMARY_FIELDS, "aggregate summary")

    evaluation_bundle._non_negative_int(summary["model_count"], "model_count")
    schemas = evaluation_bundle._bounded_list(summary["schemas_present"], "schemas_present")
    if schemas != sorted(set(schemas)):
        raise ValueError("schemas_present must be sorted and unique")
    for schema in schemas:
        if schema not in evaluation_bundle.SUPPORTED_SOURCE_SCHEMAS:
            raise ValueError("schemas_present contains unsupported schema")

    models_with_score_rows = evaluation_bundle._bounded_list(
        summary["models_with_score_rows"], "models_with_score_rows"
    )
    if models_with_score_rows != sorted(set(models_with_score_rows)):
        raise ValueError("models_with_score_rows must be sorted and unique")
    for model_id in models_with_score_rows:
        evaluation_bundle._required_model_id(model_id, "models_with_score_rows")
    if summary["deployment_allowed"] is not False:
        raise ValueError("aggregate deployment_allowed must be false")


def _validate_safety_flags(flags: Any) -> None:
    if not isinstance(flags, Mapping):
        raise ValueError("safety_flags must be an object")
    evaluation_bundle._require_exact_fields(flags, SAFETY_FLAG_FIELDS, "safety flags")
    expected = {
        "local_only": True,
        "strict_json_loaded": True,
        "derived_from_evaluation_bundle_only": True,
        "input_paths_copied": False,
        "source_filenames_copied": False,
        "raw_identifiers_copied": False,
        "generated_artifact_references_copied": False,
        "secrets_detected": False,
        "report_payload_copied": False,
        "live_capture_used": False,
        "external_services_used": False,
        "deployment_allowed": False,
    }
    if dict(flags) != expected:
        raise ValueError("safety_flags must match the v0 local-only false-claim guard")


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Generate local synthetic-only model registry metadata."
    )
    parser.add_argument("output", help="Path to write model_registry_metadata.v0 JSON")
    parser.add_argument("bundle", help="Local model_evaluation_bundle.v0 JSON")
    args = parser.parse_args(argv)

    metadata = generate_registry_metadata(load_evaluation_bundle(args.bundle))
    dump_metadata(metadata, args.output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
