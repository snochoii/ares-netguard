# Time-Series Foundation Residual Anomaly

## Idea

Use pretrained or large-scale time-series forecasting models to predict future network feature windows. Convert forecast residuals into anomaly evidence.

## Why

Network behavior is temporal. A single feature row may look normal, while its change relative to expected sequence is anomalous.

## Candidate model families

- TimesFM-style forecasting.
- Chronos-style forecasting.
- Moirai-style forecasting.
- PatchTST/iTransformer/TimeMixer-style research models.
- Conformal prediction wrappers over forecast residuals.

## Product approach

```text
host feature sequence
  -> forecast next window
  -> compare actual values
  -> residual score
  -> prediction interval breach
  -> anomaly evidence
```

## Output schema

```text
src_ip
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

## Safety

- No raw packet payloads.
- No private absolute paths.
- No unbounded model downloads in CI.
- External pretrained model use must be optional and documented.
- Tests use tiny synthetic sequences or mocked model outputs.
