# Evidence Index

`evidence_index.v0` is a local, pointer-only index over already-generated
synthetic ARES NetGuard reports. It gives downstream runtime/UI work a stable
way to discover which source rows relate to an `(entity_id, window_start)`
without copying report payloads, filenames, paths, private telemetry, model
artifacts, capture claims, deployment claims, or external-service claims.

The v0 implementation is Python stdlib only:

```bash
python -m ares_netguard.storage.evidence_index \
  /tmp/ares-netguard/evidence-index.json \
  /tmp/ares-netguard/telemetry-feature-windows.json \
  /tmp/ares-netguard/detector-zoo-feature-windows.json \
  /tmp/ares-netguard/model-disagreement-report.json \
  /tmp/ares-netguard/time-series-residual-report.json \
  /tmp/ares-netguard/traffic-representation-report.json \
  /tmp/ares-netguard/temporal-security-graph-report.json \
  /tmp/ares-netguard/agentic-investigation-report.json \
  /tmp/ares-netguard/detection-candidate-report.json \
  /tmp/ares-netguard/composed-model-score-rows.json \
  /tmp/ares-netguard/model-registry-metadata.json
```

## Supported Inputs

- `telemetry_feature_window_report.v0`
- `model_disagreement_report.v0`
- `time_series_residual_report.v0`
- `traffic_representation_report.v0`
- `temporal_security_graph_report.v0`
- `agentic_investigation_report.v0`
- `detection_candidate_report.v0`
- `model_registry_metadata.v0`
- JSON lists containing `model_score_row.v0`

Each input is loaded with strict JSON constants and validated through the
existing source contract before indexing.

The fixture smoke path passes both synthetic feature reports and the composed
score-row list as the single `model_score_rows.v0` source. It does not pass
detector-zoo or native rows separately to the bundle or evidence index; both
remain intermediate inputs to the composer.

## Output Shape

The index contains:

- generated `source_name` values derived only from schema and occurrence count;
- `source_summaries`, sorted by generated source name, with schema, counts,
  feature names, and model IDs;
- `entity_window_index` rows keyed by `entity_id` and `window_start`;
- source row pointers with `source_schema`, `row_index`, feature names, model
  IDs, and optional numeric evidence indexes;
- aggregate counts and local-only false-claim safety flags.

Registry metadata contributes source and model summaries only because it has no
entity/window rows.

## Non-Claims

`evidence_index.v0` is not a database, durable storage provider, live capture
path, PCAP parser, deployment gate, model promotion workflow, Qt binding,
external enrichment service, or native runtime executor. It is a bounded
contract and fixture-smoke path that can later migrate into the Rust runtime
once the schema is stable.

## Fixture Smoke

`make fixture-smoke` writes `/tmp/ares-netguard/evidence-index.json` and
validates that it is strict JSON. The index does not execute detectors or make
new detector claims; it points to composed rows that now include an executed
PyOD/River synthetic research run. It is not durable storage, a native runtime
execution trace, capture output, or deployment state. All outputs are runtime
artifacts and must not be committed.
