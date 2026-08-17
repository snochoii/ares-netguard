# Time-Series Foundation Residual Anomaly

## Current status

ARES has two offline forecast paths behind one numeric-only seam:

- `rolling_mean_proxy` remains the dependency-free default and emits
  `time_series_residual_report.v1`;
- `chronos_bolt_tiny_local` executes a pinned, operator-provisioned
  Chronos-Bolt-Tiny artifact and emits `time_series_residual_report.v2`.

The v0 report remains a strict read-only input, while the default proxy
continues producing v1. Residual rows and `model_score_row.v0` are unchanged.
The expanded comparison emits `time_series_forecast_evaluation.v1` and keeps
the v0 comparison readable.
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
or invokes a service. The backend does not require a child process. The B1
operator launcher deliberately creates two isolated replay processes as
described below. The local model path is never copied into analytical evidence.

## Immutable pin, lock, and manifest owners

The B1 gate consumes the existing owners without changing or regenerating
them:

- `src/ares_netguard/models/time_series_forecast.py` owns
  `CHRONOS_MODEL_ID`, `CHRONOS_MODEL_REVISION`, `CHRONOS_*_SHA256`,
  `CHRONOS_RUNTIME_PLATFORM`, and `CHRONOS_PACKAGE_VERSIONS`;
- `src/ares_netguard/models/manifests/chronos_bolt_tiny_v1.json` owns
  `model_id`, `revision`, `runtime_platform`, `packages`, and the exact
  `files[].name`, `files[].size_bytes`, and `files[].sha256` values;
- `requirements-foundation-forecast.in` owns the ten direct package pins;
- `requirements-foundation-forecast.lock` owns all 41 resolved
  `name==version` requirements and their allowed `--hash=sha256:...` wheel
  hashes;
- `pyproject.toml` owns `project.requires-python` and the matching
  `project.optional-dependencies.foundation-forecast` direct pins.

`scripts/verify_foundation_forecast_environment.py` verifies the SHA-256
identity of those four repository inputs, the exact installed package set and
versions, one locked wheel per package, every selected wheel hash, the exact
two-file model set, every model size and hash, the model revision, the bundle
digest, the CPython 3.12/Linux x86-64 CPU platform, and `pip check`. Missing,
extra, or mismatched inputs block replay.

The immutable source identities consumed by B1 are:

```text
requirements-foundation-forecast.in: 29ec8a1dbe075e9d49ff6511fc01a627f21d5fe79f0a461ab44919c30b7cd60d
requirements-foundation-forecast.lock: 59301d09b3efc73a20466d599d9a56a9634d0c42e001d41947eab2323b677b2e
pyproject.toml: c5c0df00a1767ef8b63607831e6e6eee45f293bfc110572f1aa39912f3c701ef
src/ares_netguard/models/manifests/chronos_bolt_tiny_v1.json: 93d6747444e8db71f7aaa9baf601cfdb67f8860b4fb6d953c0203189581dec3a
```

## Isolated optional dependency lane

The base environment and `make verify` do not install PyTorch or Chronos.
`requirements-foundation-forecast.in` pins the direct CPython 3.12/Linux
x86-64 CPU surface; `requirements-foundation-forecast.lock` hash-locks the
complete resolved environment. The resolver-backed tokenizers pin is `0.22.2`,
which satisfies Transformers 5.12.1's declared range.

Provisioning uses outbound network access only to obtain an allowed wheel for
each already-locked requirement and the two files at revision
`a0e552de83495b5c28c14c71c374f3e33280b340`. The operator creates one fresh
`mktemp -d /tmp/ares-netguard-b1.XXXXXX` root, a provisioning virtual
environment, wheelhouse, model directory, replay virtual environment, caches,
install report, attestation, and reports below that root. The replay
environment is installed without indexes from the wheelhouse with
`--require-hashes` and a pip `--report`:

```bash
python3.12 -m venv /tmp/ares-netguard-b1.ABC123/replay-venv
/tmp/ares-netguard-b1.ABC123/replay-venv/bin/python -m pip install \
  --no-index \
  --find-links /tmp/ares-netguard-b1.ABC123/wheelhouse \
  --require-hashes \
  --report /tmp/ares-netguard-b1.ABC123/install-report.json \
  -r requirements-foundation-forecast.lock
```

`ABC123` is a concrete `mktemp` suffix, not a literal path. Provisioning never
installs into or reuses the repository `.venv`, system packages, user site, or
user-global model caches. It does not repin anything. No wheel, model, cache,
generated report, install report, attestation, or local environment may be
committed. `*.safetensors` is globally ignored and rejected by the artifact
guard.

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

## Held-out comparison and drift replay

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

The regular-grid v1 replay has two series with 336 hourly observations each:
64 initial history, 32 frozen calibration observations, and six consecutive
40-observation scored regimes. That is 672 input rows and 480 scored rows.
The deterministic transformations use base signals
`100 + 8*sin(2*pi*i/24)` and `30 + 4*sin(2*pi*i/12)`, rounded to six decimal
places. Regime-local anomalies alternate positive and negative offsets of 48
and 24 respectively:

| Regime | Indexes per series | Exact transformation | Labels / scored | Digest |
|---|---:|---|---:|---|
| `stationary_reference` | 96-135 | unchanged base signal | 8 / 80 (0.10) | `a87d871497287bac7f7bdf0a167d1c6b860a61561f58366e58fca6e4d83ce31a` |
| `abrupt_drift` | 136-175 | level offset +24 / +12 | 8 / 80 (0.10) | `118a44658e5b14729a06fd56d5c9d25160a0fb8cb14459caa3999946800b3381` |
| `gradual_drift` | 176-215 | linear +24 to -24 / +12 to -12 over 40 points | 8 / 80 (0.10) | `14d015c502abc7240b9ea36c9cba340e774c0b44fb4b417ad016176978d91639` |
| `variance_expansion` | 216-255 | seasonal amplitude 8 to 24 / 4 to 12 | 8 / 80 (0.10) | `c1fbef59ed416f6d3e8e3a430cec4246fb68f9360732d92d7e1f1a6ba1c33240` |
| `seasonality_change` | 256-295 | period 24 to 12 / 12 to 6, cosine phase | 8 / 80 (0.10) | `969247467ad3e3ce3bd56d81fd91d2affba3842f939e155f892ffb3045af1c6d` |
| `anomaly_density_change` | 296-335 | base signal; anomaly offsets 1,5,...,37 | 20 / 80 (0.25) | `e9965032f1ead00e9c1adb77d06c51197e02a2394793d29639fafe2f64c308ed` |

The first five regimes use anomaly offsets 7, 17, 29, and 37 per series. The
last regime uses 1, 5, 9, 13, 17, 21, 25, 29, 33, and 37, so anomaly density
really changes from 0.10 to 0.25. There are 60 labels in total. The cohort
digest
`588231a1a3f0129aa680bdf87d2071118fa2336274e8bd1979a4ac72b4cda7a7`
binds the cohort ID, regime descriptors, all normalized rows, and all sorted
labels.

The existing input and forecast contracts require finite observations on the
frozen regular grid; they provide no imputation, interpolation, or cadence
normalization. Consequently, two derived stress cohorts are rejection
evidence, not inference cohorts:

- `missing_observation` removes `fixture-a/bytes_out` index 150, contains 671
  rows, and must return `missing_observation_gap` before scoring with zero
  inference calls; its digest is
  `c3ce59e87bbb5941ca37eeac8d1e55c1e918901ef1808a2a240b40b3f43ec0c8`;
- `irregular_timestamp_interval` moves `fixture-b/connection_rate` index 150
  from the hour to `+30` minutes, contains 672 rows, and must return
  `irregular_timestamp_interval` before scoring with zero inference calls; its
  digest is
  `58a66e89bf67215b4347690f2e581b108e64679038b738426ee73bf2f035ec15`.

Each stress digest binds its case ID, source cohort digest, all transformed
rows, all labels, and expected rejection marker. The v1 analytical report
records per-regime and aggregate metrics, the two rejection results, safety
flags, and explicit non-claims. The `/tmp`-only
`time_series_forecast_replay_analytical.v1` bundle additionally contains the
complete 672-row digest input cohort, both complete residual reports, and the
labels. Validation recomputes the pinned cohort digest and binds each scored
key and actual value to both residual reports. It also checks the serialized
forecast/residual relationship within `0.000002`, solely to account for the
existing six-decimal residual-row serialization; this is not used to relax
two-run comparison, which remains exact. Comparison covers keys, actual
values, forecasts, intervals, residuals, anomaly scores, labels, metrics,
digests, safety flags, and non-claims. The outer real-model gate additionally
requires both run bundles to equal the pinned canonical analytical digest
`d3acdeacaee03fcc1a95acb5a351d0ffe6af953e2e49a9b06c764b909a3777aa`;
coordinated changes that remain structurally self-consistent therefore fail
the pinned-output identity check.

Operational measurements are deliberately separate in
`time_series_forecast_replay_operational.v0`. Each run records all 544
per-backend inference latencies in nanoseconds, replay runtime in nanoseconds,
and peak replay-process RSS in KiB using
`resource.getrusage(RUSAGE_SELF).ru_maxrss`. Validation requires the values to
be present, integral, non-negative (positive for runtime and RSS), correctly
counted, and paired with fixed units and methods. Timing and RSS are never part
of exact equality and ordinary variation is not model nondeterminism.

## Offline and process-isolation gate

Every cache root used by the replay is created below the isolated B1 root:
`HOME`, `XDG_CACHE_HOME`, `PIP_CACHE_DIR`, `HF_HOME`, `HF_HUB_CACHE`,
`HUGGINGFACE_HUB_CACHE`, `HF_XET_CACHE`, `HF_ASSETS_CACHE`,
`HF_MODULES_CACHE`, `TRANSFORMERS_CACHE`, `TORCH_HOME`,
`TORCH_EXTENSIONS_DIR`, `TORCHINDUCTOR_CACHE_DIR`, `TRITON_CACHE_DIR`,
`MPLCONFIGDIR`, `PYTHONPYCACHEPREFIX`, and `TMPDIR`.
`PYTHONNOUSERSITE=1`, a repository-`src`-only `PYTHONPATH`, and
`PIP_CONFIG_FILE=/dev/null` prevent user site, inherited Python paths, and pip
configuration reuse.

`HF_HUB_OFFLINE`, `TRANSFORMERS_OFFLINE`, cache isolation, and
`local_files_only=True` are defense in depth, not the offline proof. The outer
launcher starts each replay through `/usr/bin/unshare --user --map-root-user
--net`. In the same inner process that loads and scores the model, a canary
must prove that the network namespace differs from the parent, `/proc/net/dev`
contains only `lo`, and a TCP connection to `1.1.1.1:443` fails. A failed
namespace check or a successful canary emits `B1_REAL_MODEL_GATE: blocked`.
Because model verification requires the exact local file set before import and
the process has no external interface, a cache miss fails rather than
downloads.

Provisioning tools and the outer launcher are outside the replay boundary. The
outer launcher creates exactly two independent replay children. There is no
claim of OS-level subprocess denial. Inside each child, a Python audit hook and
monkeypatched `socket`/`subprocess` entry points reject Python-level socket or
process attempts; expected child count is zero. Chronos/Torch native threads
are allowed and pinned to one compute thread. Any unexpected native descendant
would still inherit the network namespace. No library subprocess is required;
an attempted one fails the run.

The explicit real-model gate writes only below the isolated `/tmp` root,
repeats the full analytical run, validates separate operational evidence, and
fails if the model path appears in evidence:

```bash
make verify-foundation-forecast \
  B1_ISOLATED_ROOT=/tmp/ares-netguard-b1.ABC123 \
  B1_WHEELHOUSE=/tmp/ares-netguard-b1.ABC123/wheelhouse \
  B1_INSTALL_REPORT=/tmp/ares-netguard-b1.ABC123/install-report.json \
  FOUNDATION_PYTHON=/tmp/ares-netguard-b1.ABC123/replay-venv/bin/python \
  CHRONOS_MODEL_ROOT=/tmp/ares-netguard-b1.ABC123/model
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
