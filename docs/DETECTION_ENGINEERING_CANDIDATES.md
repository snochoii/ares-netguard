# Detection Engineering Candidates

## Goal

Use recurring ML evidence to propose candidate detection rules.

## Sources

- repeated model disagreement patterns
- recurring residual anomalies
- repeated graph rare-edge patterns
- confirmed analyst labels
- Falco/Suricata/Zeek evidence co-occurrence

## Outputs

- Zeek script candidate
- Sigma-like query candidate
- Suricata local rule draft
- SIEM query candidate
- validation report

## Safety rules

- Never deploy generated rules automatically.
- Candidate rules must be reviewed.
- Candidate rules must be tested against fixtures/replay data.
- False-positive estimate must be reported.
- Generated rules are drafts, not authoritative detections.
