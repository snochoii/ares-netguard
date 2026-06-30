# Self-Supervised Traffic Representation

## Goal

Move beyond handcrafted flow features by learning representations from packet/flow sequences.

## Candidate approaches

- ET-BERT-style datagram representation.
- Packet sequence contrastive learning.
- Masked autoencoder over flow/packet feature tensors.
- NetMamba-inspired efficient traffic encoders.
- Embedding-based novelty detection.

## Staged implementation

1. Tokenize sanitized packet/flow sequence metadata.
2. Build toy embedding pipeline over synthetic fixtures.
3. Add contrastive/MAE research experiment under `python_lab/`.
4. Store embeddings as generated artifacts, never committed.
5. Compare embedding anomaly score against tabular model scores.
6. Export stable encoders to ONNX when feasible.

## Privacy boundary

Do not store raw payloads, credentials, URLs with sensitive query strings, full packet bodies, or private PCAP content in model reports.
