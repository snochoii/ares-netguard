# Time-Series Foundation Residual Anomaly

## Current status

`time_series_residual_report.v1` is an offline, stdlib-only research contract
for leakage-resistant forecast-residual evidence. It introduces an explicit
forecast backend seam and frozen split-conformal calibration. It is not a real
TimesFM, Chronos, Moirai, PatchTST, iTransformer, TimeMixer, or other pretrained
foundation model execution path.

The former `time_series_residual_report.v0` contract remains accepted as a
strict read-only input by score conversion, composition, evaluation, agentic
investigation, evidence indexing, and Rust schema allowlists. New reports are
always v1.

## Forecast backend seam

`src/ares_netguard/models/time_series_forecast.py` defines:

- `ForecastRequest(context: tuple[float, ...], interval_z: float)`;
- `ForecastEstimate(mean, lower, upper, scale)`;
- a fixed-identity `ForecastBackend` protocol with immutable settings and one
  `forecast_one()` operation;
- an immutable `ForecastBackendSafety` contract from which report safety flags
  are derived and validated before execution;
- a closed resolver whose only CLI name is `rolling_mean_proxy`.

The built-in backend identity is `rolling_mean_proxy_v1`, version `1`, kind
`deterministic_proxy`. It calculates the arithmetic mean and population
standard deviation of the supplied context, with a `1e-6` minimum scale.

The backend request deliberately excludes entity IDs, feature names,
timestamps, file paths, telemetry payloads, the current target, and future
values. Selectors that look like URLs, paths, dotted imports, model-hub IDs, or
unknown names are rejected without fallback. The adapter contains no dynamic
imports, sockets, subprocesses, cache discovery, weight lookup, or downloads.
Programmatic mock adapters must explicitly attest the same offline,
synthetic-only, no-pretrained-model, no-artifact, no-network, no-download,
no-external-service, and no-deployment contract. Missing or false safety claims
fail before the first forecast; custom adapter code remains trusted local code
and is not sandboxed by this seam.

## Frozen calibration flow

Defaults are `history_window=3`, `calibration_window=8`, and `interval_z=2.0`.
Each entity/feature series is processed independently:

1. The first three values form forecast context.
2. The next eight targets are forecast before observation, producing
   standardized absolute residuals.
3. Those eight calibration scores are frozen.
4. Later targets are scored against only that frozen cohort.
5. With target score `s`, the p-value is
   `(1 + count(calibration_score >= s)) / 9` and the conformal anomaly score is
   `1 - p`.
6. Residual risk remains the maximum of conformal score, bounded z-risk, and a
   `0.75` interval-breach floor.
7. A current actual value enters history only after its forecast and score are
   complete.

The first evidence row is therefore emitted on observation 12. Every input
series must contain at least 12 observations. Conservative `>=` ties,
finite-sample correction, score-before-observe ordering, and no-future-data
behavior are explicit v1 metadata.

## Report contract

The top-level v1 report contains:

```text
schema_version
model_id
model_family
history_window
calibration_window
interval_z
forecast_backend
calibration
safety_flags
rows
```

The strict nested metadata records backend identity/version/kind/settings,
calibration method/count/frozen state/tie rule, and local synthetic safety
non-claims. Residual row fields remain unchanged from v0:

```text
entity_id
feature_name
window_start
actual_value
forecast_mean
forecast_lower
forecast_upper
residual
residual_z
conformal_score
residual_risk
model_id
model_family
```

Conversion still emits `model_score_row.v0` with model ID
`time_series_residual`. V1 score evidence preserves every feature row and adds
one structured backend/calibration provenance object per entity/window score.

## Input, output, and safety boundaries

- Inputs are bounded strict JSON/JSONL with exact fields, coarse synthetic
  entity IDs, snake_case feature names, finite values, UTC timestamps, and
  duplicate/order checks.
- Backend estimates must be finite, use positive scale, and satisfy
  `lower <= mean <= upper`; failures do not select a fallback.
- A complete report is validated before atomic replacement of an output;
  lexical and resolved repository locations are checked, and symlink outputs
  are rejected.
- Repository outputs are limited to ignored `data/reports/`, `.runtime/`, or
  `artifacts/` roots. Fixture smoke writes to `/tmp/ares-netguard/`.
- Tests use synthetic fixtures only. No live capture, raw packet payloads,
  private telemetry, external service, pretrained weights, artifact
  persistence, download, or deployment is involved.

Fixture smoke invocation:

```bash
python -m ares_netguard.models.time_series_residual \
  tests/fixtures/time_series_residual/synthetic_windows.jsonl \
  /tmp/ares-netguard/time-series-residual-report.json \
  --backend rolling_mean_proxy \
  --history-window 3 \
  --calibration-window 8
```

## Technology and migration

Selected technology is Python stdlib for the experimental forecast/calibration
adapter, with a minimal Rust source-schema allowlist update for typed handoff
compatibility. Rust does not implement this unstable research backend; C++ and
Qt/QML add no behavior to this milestone, and the static QML/v0 snapshot stays
unchanged.

Migration path:

```text
offline deterministic proxy or mock
  -> optional locally provisioned backend with immutable revision/digest
  -> reproducible held-out evaluation
  -> stable export contract
  -> ONNX or selected native-runtime candidate
```

V1 improves reproducibility and leakage resistance, but remains synthetic,
in-memory, download-disabled, non-persistent, and deployment-disabled.
