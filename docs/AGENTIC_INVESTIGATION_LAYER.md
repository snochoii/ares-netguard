# Agentic Investigation Layer

## Purpose

Agents are not primary detectors. They are evidence-grounded investigation assistants.

## Allowed behavior

- Generate hypotheses from model disagreement and evidence.
- Ask bounded, read-only queries over local artifacts.
- Gather supporting and refuting evidence.
- Build an investigation graph.
- Draft MITRE mapping and response guidance.
- Suggest follow-up queries.

## Forbidden behavior

- No live probing without explicit operator authorization.
- No destructive action.
- No rule deployment.
- No autonomous final incident classification.
- No exfiltration of telemetry to external services unless explicitly configured and approved.

## Required contracts

Every investigation output must include:

```text
hypothesis_id
claim
supporting_evidence_refs
refuting_evidence_refs
missing_evidence
confidence
recommended_next_query
human_review_required
```

## First milestone

- Offline investigation over existing generated artifacts.
- Deterministic tests.

## v0 offline report generator

`agentic_investigation_report.v0` is a stdlib-only local report generated from
existing synthetic evidence artifacts. It does not use an LLM, make network
requests, discover files, probe systems, enrich indicators, deploy rules, or
decide whether an incident is real.

CLI:

```bash
python -m ares_netguard.investigation.agentic_layer \
  /tmp/ares-netguard/model-disagreement-report.json \
  /tmp/ares-netguard/agentic-investigation-report.json \
  --evidence-report /tmp/ares-netguard/time-series-residual-report.json \
  --evidence-report /tmp/ares-netguard/traffic-representation-report.json \
  --evidence-report /tmp/ares-netguard/temporal-security-graph-report.json
```

Primary input:

- `model_disagreement_report.v0`

Optional local evidence inputs:

- `time_series_residual_report.v0`
- `traffic_representation_report.v0`
- `temporal_security_graph_report.v0`

The generator matches optional evidence rows only by `(entity_id,
window_start)`. It never scans directories or expands arbitrary input sources.

## v0 schema

Top-level report:

```text
schema_version
primary_report_schema
evidence_report_schemas
rows
```

Each row in `rows` has exactly:

```text
hypothesis_id
claim
supporting_evidence_refs
refuting_evidence_refs
missing_evidence
confidence
recommended_next_query
human_review_required
```

`human_review_required` is always `true`.

Evidence references point back to source report locations instead of copying
evidence blobs. A reference includes:

```text
report_schema
entity_id
window_start
model_id
row_index
field_path
evidence_index, when referencing an evidence-list entry
```

## v0 hypotheses

The deterministic v0 rules emit bounded hypotheses for:

- high consensus risk with multiple supporting models;
- high disagreement when an outlier model is present;
- sparse or missing supporting evidence that requires follow-up;
- optional local evidence rows that match the disagreement row entity/window.

Confidence is a deterministic ranking aid only. It is not incident probability
and is not an autonomous classification.

## Guardrails

The v0 loader rejects:

- unknown schemas;
- non-strict JSON constants such as `NaN` and `Infinity`;
- non-finite numbers;
- oversized strings, lists, mappings, or deeply nested payloads;
- raw IPs, domains, URLs, emails, private paths, command lines, secrets, and
  payload fields;
- directory inputs or implicit file discovery.

The module performs no external or network actions.

## Non-claims

This layer is not:

- an autonomous SOC agent;
- a live investigation system;
- an external LLM workflow;
- a probing or enrichment tool;
- a final incident classifier.

## Follow-up path

The intended migration path is:

```text
deterministic local investigation report
  -> stable evidence reference schema
  -> bounded read-only query and retrieval tools
  -> analyst feedback and evaluation reports
  -> product runtime/storage integration after schemas stabilize
```
