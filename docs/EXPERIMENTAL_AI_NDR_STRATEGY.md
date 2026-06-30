# Experimental AI-NDR Strategy

ARES NetGuard-ML should focus on adding experimental AI capability on top of network and host telemetry, not on recreating a conventional NDR.

## Product thesis

Existing platforms hide most model internals. ARES NetGuard-ML exposes the model lab:

- which model fired;
- which model did not fire;
- why the models disagree;
- what telemetry evidence supports the hypothesis;
- what follow-up data would confirm or refute it;
- whether a detector can be turned into a reusable rule;
- whether the model can be exported to a stable native runtime.

## Core loop

```text
telemetry
  -> feature/evidence store
  -> model zoo + online + graph + foundation residual detectors
  -> model disagreement engine
  -> evidence-grounded investigation
  -> detection candidate generation
  -> evaluation report
  -> native inference / analyst workflow
```

## Experimental tracks

1. Model Disagreement Engine
2. Time-Series Foundation Residual Anomaly
3. Self-Supervised Traffic Representation
4. Temporal Heterogeneous Security Graph
5. Agentic Investigation Layer
6. Detection Engineering Candidate Generator
7. Native Inference Adapters

## Rule

Every experimental capability must produce:

- testable artifacts;
- reproducible evaluation;
- privacy-safe outputs;
- schema contracts;
- a rollback path;
- a clear statement that it is experimental until validated.
