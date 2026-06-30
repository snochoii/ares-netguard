# Temporal Heterogeneous Security Graph

## Goal

Represent security telemetry as a time-evolving graph and detect anomalous
relationships without exposing raw private indicators.

## Stdlib v0

`src/ares_netguard/graph/temporal_security_graph.py` implements
`temporal_security_graph_report.v0` from synthetic
`temporal_graph_edge_row.v0` inputs. It is a deterministic stdlib-only
baseline, not NetworkX, community detection, graph storage, GNN inference, or a
production graph runtime.

The CLI writes a stable JSON report:

```bash
python -m ares_netguard.graph.temporal_security_graph \
  tests/fixtures/temporal_security_graph/synthetic_edges.jsonl \
  /tmp/ares-netguard/temporal-security-graph-report.json \
  --history-window 3 \
  --min-history-windows 2
```

Each input row is one coarse edge observation:

- `schema_version`: `temporal_graph_edge_row.v0`
- `event_id`
- `entity_id`
- `window_start`
- `source_node_type`
- `source_node_id`
- `edge_type`
- `target_node_type`
- `target_node_id`
- optional `observed_count`

Allowed node types are `host`, `process`, `user`, `domain`, `ip`, `asn`,
`alert`, `model_signal`, `file`, and `service`. Allowed edge types are
`connected_to`, `resolved`, `spawned`, `triggered`, `co_occurred`,
`authenticated_to`, `downloaded`, `wrote_file`, `shares_destination`, and
`communicated_with`.

## Features

The v0 baseline scores each entity/window edge against a bounded rolling
history:

- `rare_edge_score`: high when the canonical edge key has not appeared in the
  history window.
- `new_neighbor_ratio`: fraction of current neighbors for the source node that
  were not present in prior windows.
- `degree_change_score`: bounded positive source-degree increase relative to
  historical mean degree.
- `graph_novelty_risk`: max of the three feature scores, rounded to six
  decimals.

Directed edges keep their direction. Symmetric `co_occurred` and
`shares_destination` edges are canonicalized so reversed observations share one
edge key. Warmup rows are emitted with zero risk until `min_history_windows`
prior windows are available.

`temporal_graph_evidence_to_score_rows(report)` folds edge evidence by
`(entity_id, window_start)` into existing `model_score_row.v0` rows under
`graph_novelty`, using the max edge risk for the window. This allows the graph
baseline to participate in the existing model disagreement report without
changing `disagreement.py`.

## Privacy Boundary

The v0 contract accepts only synthetic/coarse IDs such as `asset-alpha`,
`host-alpha`, `service-web`, or `process-browser`. It rejects raw IP addresses,
domains, URLs, usernames, email addresses, filesystem paths, payloads, command
lines, secrets, unknown fields, unknown node types, unknown edge types,
non-finite numbers, duplicate event IDs, and malformed timestamps.

Generated graph reports are runtime artifacts and must stay outside git. The
committed fixture under `tests/fixtures/temporal_security_graph/` is synthetic.

## Non-Claims

This milestone does not implement live capture, enrichment, DNS resolution,
PCAP parsing, graph databases, community detection, lateral path hypotheses,
NetworkX analytics, GNNs, or native runtime inference. It is research evidence
for model comparison and schema stabilization only.

## Roadmap

1. Stdlib rare-edge/new-neighbor/degree-change baseline.
2. Stable graph schema and evaluation reports.
3. NetworkX centrality, component, and community-change features.
4. Temporal heterogeneous graph snapshots.
5. Graph ML/GNN experiments when fixture evidence justifies them.
6. Graph evidence panel in the Qt workstation after the runtime/UI boundary is
   ready.
