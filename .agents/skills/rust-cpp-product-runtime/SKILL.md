---
name: rust-cpp-product-runtime
description: Develop Rust/C++ product runtime components for session/workspace/job/storage/capture/native inference.
---


# Rust/C++ Product Runtime

Use for product runtime, not ML experimentation.

Prioritize safety boundaries, storage contracts, process supervision, and native inference.

Do not rewrite working Python ML research or evaluation pipelines into Rust/C++
only for aesthetics. Choose this boundary for long-running services,
workspace/session/job/storage/capture safety, packaging-sensitive runtime work,
and native inference where the technology selection policy says native runtime
stability matters.
