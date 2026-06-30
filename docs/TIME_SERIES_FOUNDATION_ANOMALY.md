# Time-Series Foundation Residual Anomaly

## Idea

Use forecasting residuals to produce temporal anomaly evidence for network or
host feature windows. A single feature row may look normal, while its change
relative to an expected sequence is anomalous.

## v0 status

`src/ares_netguard/models/time_series_residual.py` is a deterministic
stdlib-only residual proxy. It is not a real TimesFM, Chronos, Moirai, PatchTST,
iTransformer, TimeMixer, or other pretrained foundation model adapter.

The v0 producer:

- reads tiny synthetic feature-window JSON/JSONL rows;
- forecasts the next value from a fixed previous-window mean and interval;
- emits residual, residual z-score, conformal-style score, and bounded
  residual risk evidence;
- converts residual evidence into existing `model_score_row.v0` rows using
  `time_series_residual` as the model id;
- writes smoke output only to `/tmp/ares-netguard/`.

## Product approach

```text
host/entity feature sequence
  -> deterministic forecast proxy
  -> compare actual value with forecast interval
  -> residual and conformal-style scoring
  -> anomaly evidence
  -> model disagreement input row
```

## v0 residual evidence row schema

Each row in `time_series_residual_report.v0` uses exactly these fields:

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

`model_id` is `time_series_residual`. `model_family` is
`experimental_time_series`.

## Future adapters

Future research milestones may add optional TimesFM, Chronos, Moirai,
PatchTST/iTransformer/TimeMixer-style, or conformal forecast adapters behind
the same evidence schema. Those adapters must remain optional and must not
download models in CI.

## Safety

- No raw packet payloads.
- No private absolute paths.
- No live capture or external probing.
- No network calls or model downloads in tests or CI.
- External pretrained model use must be optional and documented.
- Tests use tiny synthetic sequences or mocked model outputs.
- v0 residual evidence is research-only and is not a production detector claim.
