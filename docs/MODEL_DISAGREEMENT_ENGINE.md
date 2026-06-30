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
