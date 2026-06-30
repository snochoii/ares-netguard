---
name: native-inference-adapters
description: Design and implement ONNX, LightGBM native, and selected Rust/C++ inference paths for stable models.
---


# Native Inference Adapters

Separate Python training from production inference.

Every adapter needs schema version, feature column order, model metadata, and deterministic tests.

Use ONNX Runtime, LightGBM native prediction, or selected Rust/C++ runtimes only
after a model has a stable feature contract and evaluation record. Keep a
Python sidecar for experimental or non-exportable models until promotion is
justified.
