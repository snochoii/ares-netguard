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

## v0 reference adapter

The first executable milestone is a stdlib-only reference adapter:

```bash
python -m ares_netguard.native_inference.adapters \
  tests/fixtures/native_inference/manifest.json \
  tests/fixtures/native_inference/feature_rows.jsonl \
  /tmp/ares-netguard/native-inference-score-rows.json
```

The CLI writes a JSON list of existing `model_score_row.v0` rows so the output
can feed `ares_netguard.models.disagreement` without changing the disagreement
contract.

### `native_inference_manifest.v0`

The manifest is a strict promotion contract. Top-level fields must be exactly:

```text
schema_version
model_id
model_family
feature_schema_version
feature_columns
training_data_summary
evaluation_summary
calibration_summary
export_format
inference_runtime
privacy_safety_notes
adapter
```

v0 supports only:

- `schema_version: native_inference_manifest.v0`
- `feature_schema_version: feature_vector_row.v0`
- `export_format: stdlib_linear_score.v0`
- `inference_runtime: stdlib_reference`
- `adapter.kind: linear_score.v0`
- `adapter.normalization: logistic`

`feature_columns` is an ordered list of sanitized snake_case feature names.
`adapter.weights` is an ordered numeric vector aligned to `feature_columns`;
`adapter.bias` is a finite number. The risk score is:

```text
logistic(bias + sum(feature_value[i] * weight[i]))
```

### `feature_vector_row.v0`

Feature rows must contain exactly:

```text
schema_version
entity_id
window_start
features
```

`entity_id` must be a synthetic or coarse identifier such as `host-alpha`.
`window_start` must be an ISO-8601 timestamp with timezone information.
`features` must contain exactly the manifest `feature_columns`; missing and
unexpected feature keys are rejected. Feature values must be finite numbers.

## Guardrails and non-claims

v0 is a contract and fixture-smoke path only. It does not load `.onnx`, `.pkl`,
`.joblib`, `.pt`, `.db`, PCAP, Parquet, or runtime model artifacts. It does not
claim ONNX Runtime, LightGBM native prediction, Rust, C++, or product-runtime
integration.

The adapter rejects directory inputs, non-strict JSON constants such as
`NaN`/`Infinity`, unknown schemas, missing or extra manifest fields, duplicate
feature columns, unsupported export/runtime values, non-finite numbers, unsafe
raw identifiers, path-like or artifact-like fields, secret-like fields, and
payload/PCAP/command fields.

## Promotion path

```text
stdlib reference adapter
  -> stable manifest and feature-vector validation
  -> reproducible evaluation record
  -> ONNX Runtime, LightGBM native, or Rust/C++ adapter
  -> product runtime model registry and promotion workflow
```

Until a model has stable features and evaluation evidence, it remains in the
Python research or sidecar layer rather than being treated as production native
inference.
