# Model Evaluation Bundle

`model_evaluation_bundle.v0` is a local, reproducible summary over existing
synthetic ARES model reports and score rows, including rows from bounded
library-backed research execution and native-reference scoring.

It is part of the model registry and evaluation reports roadmap track, but v0
is not a registry. It is an aggregate report contract that verifies supported
schemas and emits stable counts that can be consumed by later registry, Qt/QML,
or runtime handoff work.

## Supported inputs

The v0 bundle accepts local JSON objects for these report schemas:

- `model_disagreement_report.v0`
- `time_series_residual_report.v0` (strict legacy input)
- `time_series_residual_report.v1`
- `traffic_representation_report.v0`
- `temporal_security_graph_report.v0`
- `agentic_investigation_report.v0`
- `detection_candidate_report.v0`

It also accepts JSON lists of `model_score_row.v0` rows. The fixture-smoke path
uses the composed score-row list produced by
`ares_netguard.models.score_row_composer`, which merges the handcrafted base
score rows, executable detector-zoo score rows, native reference score rows,
and report-derived residual, representation, and graph score rows before
disagreement and bundle generation.

Unknown schemas fail closed.

## Output scope

The bundle writes:

- schemas present;
- generated safe source names derived from schema and occurrence only;
- model IDs;
- per-source row, score-row, entity, window, feature, sequence, edge,
  hypothesis, and candidate counts;
- aggregate row counts by schema;
- local-only safety flags;
- explicit non-claims.

The bundle does not copy source file paths, source filenames, entity IDs,
window timestamps, evidence payloads, raw report rows, PCAP references, artifact
paths, or private telemetry.

## Safety guards

The loader uses strict JSON parsing and rejects `NaN`, `Infinity`, and
`-Infinity`. Source validation rejects:

- unsupported schemas;
- non-finite numbers;
- raw IP addresses, URLs, email addresses, domains, paths, and command-like
  strings;
- secret-like field names and secret-like values;
- generated artifact references such as PCAP, Parquet, pickle, ONNX, model,
  database, and JSONL filenames.

The CLI writes to the caller-provided output path, but ordinary repository
paths are rejected unless they are under ignored runtime roots used for reports:
`data/reports/`, `data/registry/`, `.runtime/`, or `artifacts/`. The fixture
smoke target writes to `/tmp`.

## Non-claims

`model_evaluation_bundle.v0` is not:

- a persistent model registry;
- a model promotion gate;
- live capture or telemetry ingestion;
- external enrichment;
- rule deployment;
- native runtime execution.

## Fixture smoke

`make fixture-smoke` now produces:

```text
/tmp/ares-netguard/composed-model-score-rows.json
/tmp/ares-netguard/model-evaluation-bundle.json
```

after the synthetic detector-zoo, residual, representation, graph,
native-inference score, composed disagreement, investigation, and
detection-candidate outputs.
The bundle does not execute detectors or create detector semantics; it
summarizes the composed rows, which now include a real PyOD/River research run
over synthetic features. The bundle remains neither live capture, deployment,
durable storage, nor a native runtime execution trace.
