# Patch Notes

This scaffold repositions ARES NetGuard-ML as an Experimental AI-NDR Workstation rather than an open-source NDR clone.

Key additions:

- full AGENTS.md contract;
- master `$netguard-orchestrator`;
- experimental AI-NDR domain skills;
- read-only and worktree-only subagents;
- docs for model disagreement, time-series foundation residual anomaly, self-supervised traffic representation, temporal security graph, agentic investigation, detection candidates, and native inference;
- `.gitignore` focused on generated telemetry/model artifacts.

`$netguard-orchestrator` invocation is explicit permission for Codex to choose subagents, parallel worktrees, commit/push, PR, guarded merge, and cleanup when safe.
