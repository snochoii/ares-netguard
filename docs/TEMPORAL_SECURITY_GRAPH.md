# Temporal Heterogeneous Security Graph

## Goal

Represent security telemetry as a time-evolving graph and detect anomalous relationships.

## Nodes

- host
- process
- user
- domain
- IP
- ASN
- alert
- model signal
- file
- service

## Edges

- connected_to
- resolved
- spawned
- triggered
- co_occurred
- authenticated_to
- downloaded
- wrote_file
- shares_destination

## Baseline features

- rare edge score
- new neighbor ratio
- degree change
- community change
- shared C2 endpoint candidate
- lateral path candidate
- model-signal concentration

## Roadmap

1. NetworkX feature baseline.
2. Temporal snapshot storage.
3. Graph anomaly scoring.
4. Heterogeneous GNN experiments.
5. Graph evidence panel in Qt workstation.
