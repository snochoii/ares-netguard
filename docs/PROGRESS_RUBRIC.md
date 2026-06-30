# Progress Rubric

This rubric is heuristic. It is not a production-readiness guarantee.

## Weighting

| Capability | Weight |
|---|---:|
| Safe telemetry foundation: PCAP/Zeek/Suricata/Falco fixture pipeline | 8% |
| Feature/evidence store and privacy-safe schemas | 8% |
| Baseline ML: IsolationForest/River/PyOD basics | 10% |
| Model registry and evaluation reports | 8% |
| Model Disagreement Engine | 12% |
| Time-Series Foundation Residual Anomaly | 12% |
| Self-Supervised Traffic Representation | 10% |
| Temporal Security Graph Anomaly | 10% |
| Agentic Investigation Layer | 8% |
| Detection Engineering Candidate Generator | 6% |
| Native inference path: ONNX/LightGBM/Rust-C++ | 5% |
| Qt/QML AI-NDR workstation UX | 3% |

Total: 100%

## Rules

- Count only validated, merged capabilities.
- Code without tests receives at most half credit.
- Docs-only strategy does not count as implemented capability, but improves confidence.
- Streamlit counts as developer/debug UI only.
- Capture/ingestion is necessary foundation but must not dominate the score.
- IsolationForest is only baseline ML.
- Experimental AI capabilities carry the highest weight.
- Technology selection policy is governance only. It can improve confidence in
  future route choices, but it does not add implemented capability percentage.
