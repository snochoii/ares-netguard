# Model Disagreement Engine

The Model Disagreement Engine is the first major differentiating capability.

## Goal

Instead of producing one risk score, compare multiple model perspectives.

Input examples:

- IsolationForest risk.
- PyOD detector risks.
- River online anomaly risk.
- Time-series residual risk.
- Graph novelty risk.
- Commercial NDR/SIEM/XDR alert signals.
- Suricata/Falco evidence.
- Analyst feedback.

## Outputs

```text
model_score_matrix
model_agreement_score
model_disagreement_score
consensus_risk
outlier_model
top_supporting_models
top_dissenting_models
evidence_by_model
```

## Analyst questions

- Why did ECOD/COPOD fire but LOF did not?
- Did time-series residual detect a temporal shift that tabular models missed?
- Did graph novelty detect a new relationship even when raw counts looked normal?
- Did the commercial alert fire before or after the experimental models?
- Which model family is most useful for this telemetry type?

## First implementation milestone

- Read existing model score artifacts.
- Normalize scores to common comparable risk scale.
- Emit disagreement report JSON.
- Add deterministic tests with synthetic score matrices.
- Do not retrain models in the first milestone.

## v0 report contract

The first executable milestone lives in `src/ares_netguard/models/disagreement.py`.
It consumes synthetic or precomputed model score rows and emits a deterministic
JSON report. Runtime behavior is intentionally limited to score normalization and
comparison; it does not train detectors, capture traffic, inspect packet payloads,
or call external services.

Accepted score row fixtures are JSON or JSONL with:

```json
{
  "schema_version": "model_score_row.v0",
  "entity_id": "host-alpha",
  "window_start": "2026-01-01T00:00:00Z",
  "scores": {
    "isolation_forest": {
      "risk": 0.91,
      "family": "baseline",
      "evidence": ["synthetic evidence"]
    }
  }
}
```

Rows must use the exact `model_score_row.v0` schema version. Scores must be
finite numeric risk values on `0.0..1.0`, `percentile` values on `0..100`, or
`inverted_risk` values on `0.0..1.0`. Boolean, `NaN`, and infinite values are
rejected so report JSON remains strict and reproducible.

Validation:

```bash
make verify
make fixture-smoke
pytest -q tests/unit tests/integration -k "model or registry or evaluation or pyod or river or disagreement"
```
