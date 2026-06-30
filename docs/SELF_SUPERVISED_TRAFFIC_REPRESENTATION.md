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

## v0 sanitized representation contract

`src/ares_netguard/models/self_supervised_representation.py` implements the
first bounded research milestone:

- Input rows use `traffic_sequence_row.v0`.
- Output reports use `traffic_representation_report.v0`.
- Disagreement conversion emits `model_score_row.v0` with
  `model_id = "self_supervised_representation"` and
  `family = "experimental_self_supervised"`.
- The implementation is Python stdlib only.
- Reports are deterministic and use stable sorted JSON with strict finite
  numeric values.
- Embeddings are a fixed-width deterministic hash/count sketch over sanitized
  tokens.
- `representation_risk` is bounded to `0.0..1.0` and currently reflects token
  novelty within the supplied synthetic sequence set.

This v0 milestone is not ET-BERT, a masked autoencoder, contrastive training, a
pretrained model adapter, or a production detector. It is a reproducible
evidence contract for comparing sanitized representation-style signals against
other experimental model families.

Allowed v0 token sources are coarse metadata only:

- protocol class;
- direction class;
- service class;
- destination port bucket;
- bytes bucket;
- duration bucket;
- TCP flag category;
- TLS version class;
- DNS outcome class;
- entropy bucket.

Rows with raw identifiers or private content are rejected at the input boundary.
Forbidden examples include raw IPs, domains, URLs, payloads, credentials,
usernames, private paths, packet bodies, and PCAP content.

Because `model_score_row.v0` requires an `entity_id`, v0 accepts only
synthetic/coarse entity labels such as `host-alpha`, `asset-01`, or
`sensor-lab`. It rejects plain user-style identifiers such as `alice`, raw
hostnames, addresses, and other private identifiers. Sequence IDs follow the
same rule and must use synthetic labels such as `seq-001`.

## Privacy boundary

Do not store raw payloads, credentials, URLs with sensitive query strings, full packet bodies, or private PCAP content in model reports.

Generated representation reports, embeddings, vocabularies, and model artifacts
must remain under `/tmp`, gitignored `data/`, or another non-committed runtime
location.
