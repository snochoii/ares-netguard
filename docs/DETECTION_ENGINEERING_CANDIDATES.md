# Detection Engineering Candidates

## Goal

Use recurring ML evidence to propose candidate detection rules.

## Sources

- repeated model disagreement patterns
- recurring residual anomalies
- repeated graph rare-edge patterns
- confirmed analyst labels
- Falco/Suricata/Zeek evidence co-occurrence

## Outputs

- Zeek script candidate
- Sigma-like query candidate
- Suricata local rule draft
- SIEM query candidate
- validation report

## Safety rules

- Never deploy generated rules automatically.
- Candidate rules must be reviewed.
- Candidate rules must be tested against fixtures/replay data.
- False-positive estimate must be reported.
- Generated rules are drafts, not authoritative detections.

## v0 CLI

Detection Engineering Candidates v0 is an offline, stdlib-only generator over
local synthetic `model_disagreement_report.v0` JSON.

```bash
python -m ares_netguard.detection_engineering.candidates \
  /tmp/ares-netguard/model-disagreement-report.json \
  /tmp/ares-netguard/detection-candidate-report.json
```

The CLI rejects directory inputs, unknown source schemas, non-strict JSON
constants such as `NaN` and `Infinity`, non-finite numbers, oversized values,
raw IPs/domains/URLs/emails/private paths/command-line fragments/secrets, and
payload-like fields. It does not read packet captures, discover files, enrich
indicators, call external services, probe networks, or deploy rules.

## v0 Schema

The output report uses `detection_candidate_report.v0`:

```text
schema_version
source_report_schema
validation_summary
rows
```

Each candidate row uses exactly these fields:

```text
candidate_id
candidate_language
candidate_kind
title
draft
source_evidence_refs
validation
false_positive_estimate
human_review_required
deployment_allowed
```

Candidate languages are `zeek`, `sigma_like`, `suricata_local`, and
`siem_query`. Candidate kinds are:

- `high_consensus_risk`: `consensus_risk >= 0.75` and at least two model
  scores `>= 0.7`.
- `high_model_disagreement`: `disagreement_score >= 0.5` with a non-empty
  `outlier_model`.

Draft text must include `DRAFT_DO_NOT_DEPLOY`. Drafts may include only
synthetic entity IDs, model IDs, window timestamps, thresholds, and candidate
metadata. Evidence blobs are not copied into candidate rows; candidates cite
`source_evidence_refs` back into the source report.

## Validation Semantics

v0 validation means the source report schema, privacy guards, candidate field
contract, evidence-reference shape, fixture-smoke JSON persistence, and draft
review/deployment flags were checked. `validated_against_replay` is always
`false` in v0 because no replay corpus or false-positive benchmark is executed.

`false_positive_estimate` is a conservative label for analyst triage, not a
measured production false-positive rate.

## Non-Claims

The v0 generator does not create authoritative Zeek, Sigma, Suricata, or SIEM
detections. It does not prove malicious activity, does not evaluate live
traffic, and does not validate candidates against production telemetry.

## Deployment Prohibition

Every row sets `human_review_required: true` and `deployment_allowed: false`.
Generated candidates must remain drafts until an analyst reviews the evidence,
tests the candidate against fixture/replay data, and explicitly promotes a
separate reviewed rule artifact outside this v0 generator.
