# Model Registry Metadata

`model_registry_metadata.v0` is a local, synthetic-only metadata contract
derived from a validated `model_evaluation_bundle.v0`.

It is part of the model registry and evaluation reports roadmap track. The v0
contract improves model traceability by listing which sanitized model IDs were
observed in synthetic evaluation sources and whether those sources included
score rows.

## Input

The CLI accepts one local `model_evaluation_bundle.v0` JSON file:

```bash
python -m ares_netguard.models.registry_metadata \
  /tmp/ares-netguard/model-registry-metadata.json \
  /tmp/ares-netguard/model-evaluation-bundle.json
```

The loader uses strict JSON parsing and validates the bundle before deriving
metadata. Unknown bundle schemas fail closed.

## Output scope

The metadata emits one entry per bundle model ID with only safe aggregate
fields:

- `model_id`
- `registry_state: observed_synthetic_only`
- `promotion_state: not_promoted`
- `observed_source_schemas`
- generated `observed_source_names`
- `source_count`
- `has_score_rows`
- `human_review_required: true`
- `deployment_allowed: false`

The aggregate summary is recomputed from entries during validation and includes:

- `model_count`
- `schemas_present`
- `models_with_score_rows`
- `deployment_allowed: false`

## Safety guards

The contract derives metadata only from bundle source summaries. It does not
copy input paths, source filenames, entity IDs, window timestamps, raw report
rows, evidence payloads, PCAP references, generated artifact paths, secrets, or
private telemetry.

Output paths inside the repository are rejected unless they are under ignored
runtime roots: `data/reports/`, `data/registry/`, `.runtime/`, or `artifacts/`.
The fixture smoke target writes to `/tmp`.

## Non-claims

`model_registry_metadata.v0` is not:

- a persistent model registry;
- a model promotion gate;
- deployment approval;
- live capture or telemetry ingestion;
- external enrichment;
- rule deployment;
- native runtime execution.

## Fixture smoke

`make fixture-smoke` now produces:

```text
/tmp/ares-netguard/model-registry-metadata.json
```

after `/tmp/ares-netguard/model-evaluation-bundle.json`.
