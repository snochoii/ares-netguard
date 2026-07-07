# Telemetry Foundation

`telemetry_feature_window_report.v0` is the first synthetic telemetry and
feature-window foundation contract for ARES NetGuard-ML.

It is a local fixture pipeline only. It does not parse PCAPs, perform live
capture, discover directories, enrich indicators, load private telemetry, call
external services, train models, deploy rules, or execute native inference.

## Input

The v0 loader accepts caller-provided JSONL rows with:

```text
schema_version: synthetic_telemetry_event.v0
source_kind: zeek_conn | zeek_dns | suricata_alert | host_runtime
entity_id
timestamp
event_count
connection_count
dns_query_count
dns_failure_count
alert_severity
bytes_in
bytes_out
duration_ms
destination_asset_id
service_name
tls_unknown
runtime_event_count
```

Identifiers must be synthetic/coarse labels such as `host-alpha` or
`asset-edge`. Raw IPs, domains, URLs, paths, secrets, command fragments, PCAP
references, model artifacts, and private telemetry strings are rejected.

## Output

The pipeline emits `telemetry_feature_window_report.v0` with 1m and 5m
`feature_vector_row.v0` rows. Each row keeps only:

- `schema_version`
- `entity_id`
- `window_start`
- `features`

The v0 feature set for this telemetry foundation producer is:

```text
window_size_minutes
event_count
connection_count
dns_query_count
dns_failure_ratio
alert_severity_sum
max_alert_severity
bytes_in_total
bytes_out_total
duration_ms_total
destination_diversity
service_diversity
tls_unknown_ratio
runtime_event_count
```

## CLI

```bash
python -m ares_netguard.ingest.telemetry_foundation \
  tests/fixtures/telemetry_foundation/synthetic_events.jsonl \
  /tmp/ares-netguard/telemetry-feature-windows.json
```

Repository output paths are rejected unless they are under ignored runtime
roots such as `data/features/`, `data/reports/`, `.runtime/`, or `artifacts/`.
Fixture smoke writes to `/tmp`.

## Non-Claims

This milestone is not:

- live capture;
- a PCAP parser;
- private telemetry ingestion;
- external enrichment;
- model training;
- deployment approval;
- native inference execution.

## Migration Path

```text
synthetic telemetry normalizer
  -> stable feature_vector_row.v0 feature contract
  -> model/evaluation bundle integration
  -> Rust storage/runtime handoff
  -> Qt analyst display
  -> owned/authorized capture wrappers after safety gates
```
