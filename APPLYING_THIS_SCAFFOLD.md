# Applying This Scaffold

This package contains a full Codex development scaffold for the Experimental AI-NDR direction.

## Safe apply

From repo root:

```bash
mkdir -p /tmp/netguard-ai-ndr-scaffold
unzip experimental-ai-ndr-codex-scaffold.zip -d /tmp/netguard-ai-ndr-scaffold
rsync -av --dry-run /tmp/netguard-ai-ndr-scaffold/experimental_ai_ndr_codex_scaffold/ ./
```

If the dry run looks right:

```bash
rsync -av /tmp/netguard-ai-ndr-scaffold/experimental_ai_ndr_codex_scaffold/ ./
```

Then validate:

```bash
make verify
git diff --check
bash scripts/check_no_generated_artifacts.sh --staged || true
bash scripts/check_no_generated_artifacts.sh --tracked || true
git status --short
```

Commit:

```bash
git add AGENTS.md docs .agents .codex .gitignore APPLYING_THIS_SCAFFOLD.md
git commit -m "docs: define experimental ai ndr codex workflow"
git push
```

## First Codex run

```text
/plan $netguard-orchestrator
```

Then:

```text
$netguard-orchestrator
```
