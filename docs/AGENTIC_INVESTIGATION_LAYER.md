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
- Mocked query tool.
- Deterministic tests.
