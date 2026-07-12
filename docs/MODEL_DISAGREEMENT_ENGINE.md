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

The disagreement report implementation lives in
`src/ares_netguard/models/disagreement.py`. It consumes synthetic score rows,
whether handcrafted, proxy-derived, native-reference, or produced by an
executed research detector, and emits a deterministic JSON report. The
disagreement module itself remains limited to score normalization and
comparison; it does not train detectors, capture traffic, inspect packet
payloads, or call external services.

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

## Executable detector zoo v0

`src/ares_netguard/models/detector_zoo.py` is the first library-backed score
producer. It loads a strict `telemetry_feature_window_report.v0`, selects only
five-minute rows, excludes `window_size_minutes` from model input, sorts the
cohort chronologically, and requires at least 32 unique entity/window
observations.

The producer executes these fixed in-memory configurations:

- PyOD ECOD and COPOD with contamination `0.1` and one worker;
- PyOD HBOS with contamination `0.1` and 10 bins;
- PyOD Isolation Forest with contamination `0.1`, 100 estimators, seed `42`,
  and one worker;
- River `MinMaxScaler | HalfSpaceTrees` with 10 trees, height 6, a 16-row
  window, and seed `42`.

PyOD batch scores use tie-aware empirical ranks over the complete synthetic
cohort. River scores are computed before each learning update and use
tie-aware expanding ranks over emitted scores only, so no future observation
affects an online risk. The first 16 rows are warmup-only and omit `river_hst`;
the first post-warmup rank is `0.5` because it is a singleton.

The CLI writes only existing `model_score_row.v0` rows:

```bash
python -m ares_netguard.models.detector_zoo \
  /tmp/ares-netguard/detector-zoo-feature-windows.json \
  /tmp/ares-netguard/detector-zoo-score-rows.json
```

Every score carries bounded synthetic-only execution provenance, fixed
settings, and feature columns. Fitted models remain in memory and are never
persisted. This is a global synthetic cohort baseline, not a per-asset adaptive
baseline, production inference service, promotion decision, or deployment
signal. It performs no capture, payload inspection, model-weight download, or
external call.

## Score row composer

`src/ares_netguard/models/score_row_composer.py` adds the v0 fixture composer
for the primary disagreement smoke path. It accepts:

- existing JSON or JSONL `model_score_row.v0` lists;
- `time_series_residual_report.v0` (strict legacy input);
- `time_series_residual_report.v1`;
- `traffic_representation_report.v0`;
- `temporal_security_graph_report.v0`.

The composer reuses the existing residual, representation, and graph converters,
then merges rows by `(entity_id, window_start)`. Scores are merged by
`model_id`, and duplicate `(entity_id, window_start, model_id)` tuples fail
closed so the disagreement input remains unambiguous.

The CLI writes a bare strict JSON list, not a wrapper object:

```bash
python -m ares_netguard.models.score_row_composer \
  /tmp/ares-netguard/composed-model-score-rows.json \
  --score-rows tests/fixtures/model_disagreement/synthetic_scores.jsonl \
  --score-rows /tmp/ares-netguard/detector-zoo-score-rows.json \
  --score-rows /tmp/ares-netguard/native-inference-score-rows.json \
  --residual-report /tmp/ares-netguard/time-series-residual-report.json \
  --representation-report /tmp/ares-netguard/traffic-representation-report.json \
  --graph-report /tmp/ares-netguard/temporal-security-graph-report.json
```

The composer does not execute detectors or add detector semantics of its own;
the smoke flow now includes score rows from the executed synthetic detector
zoo alongside handcrafted and proxy-derived sources. Composition still adds no
live capture, deployment, durable storage, native runtime execution, or
external enrichment.

Validation:

```bash
make verify
make fixture-smoke
pytest -q tests/unit/test_detector_zoo.py tests/integration/test_detector_zoo_fixture.py
pytest -q tests/unit tests/integration -k "composer or model or registry or evaluation or pyod or river or disagreement"
```
