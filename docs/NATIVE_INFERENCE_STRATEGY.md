# Native Inference Strategy

## Goal

Keep Python for research and training, but allow stable production models to run without a permanent Python dependency.

## Paths

1. ONNX Runtime for exported neural or supported classical models.
2. LightGBM native prediction for supervised models.
3. Rust/C++ implementations for simple stable detectors.
4. Python sidecar fallback for research models.

## Model promotion stages

```text
research
  -> benchmarked
  -> reproducible
  -> exportable
  -> native inference candidate
  -> production runtime model
```

## Required metadata

Every promoted model requires:

- model_id
- model_family
- feature_schema_version
- feature_columns
- training_data_summary
- evaluation_summary
- calibration_summary
- export_format
- inference_runtime
- privacy/safety notes
