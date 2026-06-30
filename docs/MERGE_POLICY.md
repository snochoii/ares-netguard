# Merge Policy

## Default

The orchestrator may merge when all automated gates pass and required read-only reviews return `MERGE_READY: yes`.

## Required review gates

| Change type | Required review |
|---|---|
| safety/privacy/capture/telemetry | netguard-product-security-reviewer |
| model/eval/native inference contracts | netguard-integration-reviewer + netguard-ml-research-architect |
| experimental AI claim docs | netguard-ml-research-architect |
| Qt/Rust/C++ product architecture | netguard-product-architect |
| agentic investigation / generated rules | netguard-product-security-reviewer + netguard-integration-reviewer |

## Auto-merge allowed

Auto-merge is allowed if:

- local validation passed;
- CI passed or no CI exists and local integration validation passed;
- no generated artifacts are staged;
- no secrets/private telemetry are staged;
- no conflict exists;
- required reviews passed;
- branch is pushed;
- PR body includes validation summary.

## Auto-merge forbidden

Do not auto-merge if:

- validation failed;
- generated artifacts are present;
- secret/private telemetry is detected;
- conflict exists;
- required review is missing or negative;
- branch includes unrelated changes;
- branch rewrites history;
- live capture/probing was added without explicit safety documentation.
