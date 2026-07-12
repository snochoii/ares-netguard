# Time-Series Foundation Residual Anomaly

## Current status

ARES has two offline forecast paths behind one numeric-only seam:

- `rolling_mean_proxy` remains the dependency-free default and emits
  `time_series_residual_report.v1`;
- `chronos_bolt_tiny_local` executes a pinned, operator-provisioned
  Chronos-Bolt-Tiny artifact and emits `time_series_residual_report.v2`.

The v0 report remains a strict read-only input, while the default proxy
continues producing v1. Residual rows and `model_score_row.v0` are unchanged.
The local pretrained path is an experimental Python ML-sidecar capability, not
product runtime inference or a deployment gate.

## Numeric-only forecast seam

`ForecastRequest` contains only `context: tuple[float, ...]` and `interval_z`.
A backend never receives entity IDs, feature names, timestamps, labels, file
paths, raw telemetry, the current target, or future values. The resolver accepts
only `rolling_mean_proxy` and `chronos_bolt_tiny_local`; URLs, model-hub IDs,
dotted imports, paths used as selectors, cache aliases, and unknown names fail
without proxy fallback.

The proxy uses context arithmetic mean and population standard deviation with
a `1e-6` scale floor. It retains the three-history/eight-calibration v1 defaults.

The Chronos adapter has fixed research settings:

- 64 past values, four 16-value Bolt patches;
- 32 frozen calibration residuals;
- one-step horizon and `interval_z=2.0`;
- q0.1/q0.5/q0.9 prediction on CPU/fp32;
- q0.5 as the point estimate;
- scale `max((q0.9-q0.1)/(2*1.2815515655446004), 1e-6)`;
- reported interval `q0.5 ± interval_z*scale`.

The legacy residual field remains named `forecast_mean`, but v2 provenance
explicitly records `point_method=median_q0_5`.

## Pinned local artifact boundary

The packaged `forecast_artifact_manifest.v0` trust anchor fixes:

```text
model: amazon/chronos-bolt-tiny
revision: a0e552de83495b5c28c14c71c374f3e33280b340
license: apache-2.0
config.json: 1120 bytes
config SHA-256: 278f0086733031635fb1c861cb01c1bad6477420c7fcb19381a2993e335785e0
model.safetensors: 34622352 bytes
weights SHA-256: 75068728d376d2bec670379eeef4bfb4d24c0cfe24d957451f8d19b447030a32
```

The model root must be an explicit absolute directory beneath `/tmp`,
`data/models/`, `.runtime/models/`, or `artifacts/models/`. It must contain
exactly those two regular files. Root/file symlinks, special files, size or
digest drift, unexpected files, pickle formats, `auto_map`, custom code, and
configuration drift are rejected before optional packages are imported. The
root and both files must be owned by the current user and must not be writable
by group or other users; those checks and digests run again after loading.

Loading uses only the verified directory with `local_files_only=True`,
`trust_remote_code=False`, `use_safetensors=True`, `device_map="cpu"`, and
`token=False` on fp32. Hugging Face and Transformers
offline/telemetry-disable controls are set before import. Application code
never downloads weights, discovers caches, reads tokens, resolves `main`,
invokes a service, or spawns a worker process. The local path is never copied
into report provenance or error artifacts.

## Optional dependency lane

The base environment and `make verify` do not install PyTorch or Chronos.
`requirements-foundation-forecast.in` pins the direct CPython 3.12/Linux
x86-64 CPU surface; `requirements-foundation-forecast.lock` hash-locks the
complete resolved environment. The resolver-backed tokenizers pin is `0.22.2`,
which satisfies Transformers 5.12.1's declared range.

Provision a wheelhouse and model bundle outside the repository. Install only
from the pre-provisioned wheelhouse:

```bash
python3.12 -m venv /tmp/ares-chronos-venv
/tmp/ares-chronos-venv/bin/python -m pip install \
  --no-index \
  --find-links /path/to/verified-wheelhouse \
  --require-hashes \
  -r requirements-foundation-forecast.lock
```

No wheel, model, cache, generated report, or local environment may be committed.
`*.safetensors` is globally ignored and rejected by the artifact guard.

## V2 evidence and frozen calibration

Each entity/feature series is independent. Forecasts occur before each target
is observed. The first 64 values form context, the next 32 standardized
absolute residuals form a frozen split-conformal cohort, and observations
97-128 are scored without modifying that cohort. For target score `s`:

```text
p = (1 + count(calibration_score >= s)) / 33
conformal_score = 1 - p
```

The current target enters history only after forecasting and scoring. Residual
risk remains the maximum of conformal score, bounded z-risk, and the `0.75`
interval-breach floor.

V2 adds strict sanitized artifact, package, quantile-mapping, and safety
provenance. It truthfully records pretrained/operator-provisioned artifact use,
verified digests, local-files-only execution, and false network/download/
external-service/remote-code/persistence/deployment flags. Score conversion
preserves feature evidence and appends one structured provenance object.

Composer, disagreement, evaluation bundle, registry metadata, investigation,
evidence index, and Rust source-schema allowlists accept v2. Static Rust/QML v0
snapshots remain unchanged compatibility gates.

## Held-out comparison

`time_series_forecast_evaluation.v0` compares proxy v1 and Chronos v2 reports
over the same two 128-observation synthetic series: 64 history, 32 calibration,
and 32 scored observations per series. The report records a fixed cohort ID and
SHA-256 over canonical full windows plus the separate eight anomaly labels.
Generation rejects cohort, label, scored-key, or scored-actual drift. Labels are
loaded only after both reports exist. The v0 cohort identity is
`time_series_foundation_synthetic_v0`; its pinned digest is
`af2a440076123497f1435ac477df200280767c1de7f07522dc58909ba4d6ade3`.

Per backend, the evaluation records count, MAE, RMSE, interval coverage, mean
interval width, AUROC, average precision, recall at risk `>=0.75`, and false
positive rate at that threshold. Deltas are Chronos minus proxy. The report is
not a superiority assertion, production benchmark, promotion gate, or
deployment approval; a negative experimental result remains valid evidence.

The explicit real-model gate denies socket and subprocess access, repeats the
Chronos run to verify determinism, writes only under `/tmp`, and fails if the
model path appears in evidence:

```bash
make verify-foundation-forecast \
  FOUNDATION_PYTHON=/tmp/ares-chronos-venv/bin/python \
  CHRONOS_MODEL_ROOT=/tmp/ares-chronos-bolt-tiny
```

Ordinary fixture smoke continues using `rolling_mean_proxy` so default CI is
dependency- and weight-free.

## Technology and migration

Python remains the research adapter and benchmark layer. Rust only recognizes
stable evidence schema identifiers; C++ and Qt/QML add no inference behavior.

```text
pinned local Python research backend
  -> reproducible held-out and drift evaluation
  -> stable forecast/export contract
  -> ONNX or selected native-runtime candidate
  -> Rust/C++ product runtime integration
```

The current capability remains synthetic, in-memory, CPU-only,
operator-provisioned, non-persistent, and deployment-disabled.
