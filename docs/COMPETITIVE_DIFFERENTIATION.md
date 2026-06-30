# Competitive Differentiation

ARES NetGuard-ML is not intended to clone existing NDR, XDR, SIEM, or AI-SOC products.

Commercial AI-NDR systems already provide behavioral analytics, anomaly detection, risk prioritization, automated triage, and integrated response workflows. ARES NetGuard-ML should instead provide an experimental AI layer that can sit above existing telemetry and alert sources.

## Positioning

ARES NetGuard-ML is:

- local-first;
- reproducible;
- transparent;
- model-comparison oriented;
- research-grade;
- analyst-facing;
- extensible to new AI/ML techniques.

It is not:

- a generic NDR replacement;
- a black-box anomaly scoring appliance;
- a Wireshark clone;
- an IsolationForest demo;
- an open-source replica of a vendor platform.

## Differentiation pillars

1. **Model disagreement**
   - Show where detectors agree and disagree.
   - Explain why time-series residuals may disagree with density detectors.
   - Compare commercial alert signals with experimental model outputs.

2. **Foundation/residual anomaly**
   - Use time-series foundation model forecasts as a residual anomaly signal.
   - Treat forecast error and prediction interval breach as security evidence.

3. **Representation learning**
   - Learn packet/flow embeddings from raw or semi-raw traffic representations.
   - Reduce dependence on handcrafted NetFlow features.

4. **Temporal graph anomaly**
   - Treat security telemetry as evolving heterogeneous graph data.
   - Detect rare edges, new communities, lateral movement paths, and shared infrastructure.

5. **Agentic investigation**
   - AI agents do not make final detection decisions.
   - Agents gather evidence, form hypotheses, map gaps, and generate investigation notebooks.

6. **Detection engineering candidates**
   - Recurring model evidence can generate candidate rules.
   - Rules must be validated against replay/fixtures and never deployed automatically.

7. **Native product path**
   - Experimental models begin in Python.
   - Stable models can migrate to ONNX/native inference.
