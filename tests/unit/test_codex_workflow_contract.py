from __future__ import annotations

import importlib.util
import sys
from copy import deepcopy
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts" / "validate_codex_workflow.py"
SPEC = importlib.util.spec_from_file_location("codex_workflow_validator", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
validator = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = validator
SPEC.loader.exec_module(validator)


def test_repository_contract_is_current() -> None:
    validator.validate_repo(ROOT)


def test_contract_fixture_matrix() -> None:
    validator.validate_fixtures(ROOT)


def test_cli_reports_static_declarations_without_claiming_runtime_enforcement(
    capsys: pytest.CaptureFixture[str],
) -> None:
    assert validator.main(["repo"]) == 0
    output = capsys.readouterr().out
    assert "static sandbox declarations only" in output
    assert "runtime enforcement not tested" in output


def test_ci_uses_immutable_actions_and_explicit_setup_python_interpreter() -> None:
    workflow = (ROOT / ".github/workflows/codex-workflow.yml").read_text(encoding="utf-8")
    assert "PYTHON=python3 make verify-codex-workflow" in workflow
    assert ".venv" not in workflow
    actions = validator.re.findall(
        r"^\s*uses:\s+([^@\s]+)@([^\s#]+)", workflow, validator.re.MULTILINE
    )
    assert [name for name, _ in actions] == ["actions/checkout", "actions/setup-python"]
    assert all(validator.re.fullmatch(r"[0-9a-f]{40}", ref) for _, ref in actions)


@pytest.mark.parametrize(
    "branch",
    ["main", "", "HEAD", "(HEAD detached at abc1234)", " codex/task", "codex//task"],
)
def test_branch_guard_rejects_protected_or_malformed_states(branch: str) -> None:
    with pytest.raises(validator.ContractError):
        validator.validate_branch(branch)


@pytest.mark.parametrize("branch", ["codex/task", "feature/task", "release/a3-validation"])
def test_branch_guard_accepts_repository_permitted_non_main_names(branch: str) -> None:
    assert validator.validate_branch(branch) == branch


def test_receipt_no_is_valid_structure_but_not_ready() -> None:
    head = "a" * 40
    parsed = validator.parse_review_receipt(f"MERGE_READY: no\nHEAD_SHA: {head}", head)
    assert parsed == {"ready": False, "head_sha": head}


def test_current_head_change_invalidates_receipt() -> None:
    receipt = f"MERGE_READY: yes\nHEAD_SHA: {'a' * 40}"
    with pytest.raises(validator.ContractError, match="stale HEAD_SHA"):
        validator.parse_review_receipt(receipt, "b" * 40)


@pytest.mark.parametrize(
    "category",
    ["product_direction", "technology_choice", "architectural_tradeoff", "product_fit"],
)
def test_all_root_owned_review_categories_are_accepted(category: str) -> None:
    packet = {
        "review_category": category,
        "objective": "One bounded decision.",
        "base_sha": "a" * 40,
        "head_sha": "b" * 40,
        "inspected_paths": ["docs/MERGE_POLICY.md"],
        "required_evidence": ["Exact evidence"],
        "decision_criteria": ["Explicit criterion"],
        "output_contract": "sha_bound_review_v1",
        "stopping_conditions": ["Stop on missing evidence"],
    }
    validator.validate_review_packet(packet)


def test_root_review_packet_cannot_carry_mutation_authority() -> None:
    packet = {
        "review_category": "product_fit",
        "objective": "One bounded decision.",
        "base_sha": "a" * 40,
        "head_sha": "b" * 40,
        "inspected_paths": ["docs/MERGE_POLICY.md"],
        "required_evidence": ["Exact evidence"],
        "decision_criteria": ["Explicit criterion"],
        "output_contract": "sha_bound_review_v1",
        "stopping_conditions": ["Stop on missing evidence"],
        "commit_authority": "authorized",
    }
    with pytest.raises(validator.ContractError, match="unexpected"):
        validator.validate_review_packet(packet)


def _valid_lane_packet() -> dict[str, object]:
    return {
        "skill_name": "test-eval-engineering",
        "skill_path": ".agents/skills/test-eval-engineering/SKILL.md",
        "objective": "One bounded lane.",
        "base_sha": "a" * 40,
        "worktree_path": "/tmp/ares-netguard-test-lane",
        "branch": "codex/test-lane",
        "owned_paths": ["src/ares_netguard/example.py"],
        "forbidden_paths": ["src/ares_netguard/other.py"],
        "required_tests": ["python3 -m pytest tests/unit/test_example.py"],
        "stopping_conditions": ["Stop on scope growth"],
        "commit_authority": "not_authorized",
        "push_authority": "not_authorized",
        "result_contract": "lane_result_v1",
    }


def test_fallback_reuses_exact_lane_packet() -> None:
    packet = _valid_lane_packet()
    validator.validate_fallback_packet(packet, deepcopy(packet))
    changed = deepcopy(packet)
    changed["objective"] = "A changed objective."
    with pytest.raises(validator.ContractError, match="unchanged packet"):
        validator.validate_fallback_packet(packet, changed)


def test_cross_lane_ancestor_overlap_is_rejected() -> None:
    first = _valid_lane_packet()
    second = deepcopy(first)
    second["branch"] = "codex/test-lane-two"
    second["worktree_path"] = "/tmp/ares-netguard-test-lane-two"
    second["owned_paths"] = ["src/ares_netguard/example.py/helpers"]
    with pytest.raises(validator.ContractError, match="cross-lane"):
        validator.validate_lane_set([first, second], ROOT)


@pytest.mark.parametrize(
    "path",
    [
        "AGENTS.md",
        ".codex/config.toml",
        ".codex/agents/netguard-lane-worker.toml",
        ".agents/skills/netguard-orchestrator/SKILL.md",
        ".agents/skills/netguard-parallel-dev/SKILL.md",
        ".agents/skills/netguard-worktree-lane-worker/SKILL.md",
        ".agents/skills/netguard-integration-merge/SKILL.md",
        ".agents/skills/git-safe-commit-push/SKILL.md",
        ".agents/skills/github-pr-create-merge/SKILL.md",
        "Makefile",
        "pyproject.toml",
        "requirements.txt",
        "requirements-dev.txt",
        "requirements-foundation-forecast.in",
        "requirements-foundation-forecast.lock",
        "scripts/check_no_generated_artifacts.sh",
        "scripts/validate_codex_workflow.py",
        "docs/MERGE_POLICY.md",
        "docs/TECHNOLOGY_SELECTION_POLICY.md",
        ".github/workflows/codex-workflow.yml",
    ],
)
def test_required_serial_chokepoints_are_classified(path: str) -> None:
    assert validator.is_shared_chokepoint(path)


@pytest.mark.parametrize("path", [".codex", ".agents/skills", ".github/workflows", "scripts"])
def test_serial_chokepoint_matching_is_ancestor_aware(path: str) -> None:
    assert validator.is_shared_chokepoint(path)


def test_component_local_build_files_are_not_promoted_to_global_chokepoints() -> None:
    assert not validator.is_shared_chokepoint("apps/rust-core/Cargo.toml")
    assert not validator.is_shared_chokepoint("apps/qt-workstation/CMakeLists.txt")


def test_read_only_lane_does_not_claim_writer_ownership() -> None:
    validator.validate_path_ownership(
        [
            {
                "name": "research-a",
                "kind": "read-only",
                "owned_paths": ["Makefile"],
                "forbidden_paths": [],
            },
            {
                "name": "research-b",
                "kind": "read-only",
                "owned_paths": ["Makefile"],
                "forbidden_paths": [],
            },
        ]
    )


def test_identifier_allowlist_is_explicit() -> None:
    valid = {"netguard-orchestrator"}
    references = {"netguard-orchestrator", "netguard-prose-category"}
    with pytest.raises(validator.ContractError, match="dangling"):
        validator.validate_identifier_references(references, valid)
    validator.validate_identifier_references(references, valid, {"netguard-prose-category"})


@pytest.mark.parametrize(
    "claim",
    [
        "This reviewer may merge the pull request.",
        "This child is authorized to commit the changes.",
        "This agent grants mutation authority.",
        "This agent owns merge-readiness.",
        "MERGE_EXECUTION: authorized",
    ],
)
def test_custom_agent_positive_authority_claims_are_rejected(claim: str) -> None:
    with pytest.raises(validator.ContractError, match="authority"):
        validator.validate_agent_authority_text("fixture-agent", claim)


def _copy_agent_catalog(tmp_path: Path) -> Path:
    target_root = tmp_path / ".codex/agents"
    target_root.mkdir(parents=True)
    for source in (ROOT / ".codex/agents").glob("*.toml"):
        (target_root / source.name).write_text(source.read_text(encoding="utf-8"), encoding="utf-8")
    return target_root


def test_agent_filename_and_name_must_match(tmp_path: Path) -> None:
    catalog = _copy_agent_catalog(tmp_path)
    path = catalog / "netguard-codebase-explorer.toml"
    path.write_text(
        path.read_text(encoding="utf-8").replace(
            'name = "netguard-codebase-explorer"', 'name = "netguard-renamed-explorer"'
        ),
        encoding="utf-8",
    )
    with pytest.raises(validator.ContractError, match="filename/name mismatch"):
        validator.load_agent_catalog(tmp_path)


def test_only_lane_worker_may_declare_workspace_write(tmp_path: Path) -> None:
    catalog = _copy_agent_catalog(tmp_path)
    path = catalog / "netguard-codebase-explorer.toml"
    path.write_text(
        path.read_text(encoding="utf-8").replace(
            'sandbox_mode = "read-only"', 'sandbox_mode = "workspace-write"'
        ),
        encoding="utf-8",
    )
    with pytest.raises(validator.ContractError, match="read-only|workspace-write"):
        validator.load_agent_catalog(tmp_path)


@pytest.mark.parametrize(
    "frontmatter",
    [
        "# no frontmatter\n",
        "---\nname: example\n---\n",
        "---\nname: example\ndescription: present\nname: duplicate\n---\n",
    ],
)
def test_invalid_skill_frontmatter_is_rejected(frontmatter: str) -> None:
    with pytest.raises(validator.ContractError):
        validator.parse_frontmatter(frontmatter, "fixture/SKILL.md")


def _copy_governance_surface(tmp_path: Path) -> None:
    for relative in validator.GOVERNANCE_SKILL_PATHS.values():
        target = tmp_path / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text((ROOT / relative).read_text(encoding="utf-8"), encoding="utf-8")


@pytest.mark.parametrize(
    ("skill", "guard"),
    [
        (
            "github-pr-create-merge",
            "Do not route reviews, parse review receipts, decide readiness,",
        ),
        (
            "netguard-worktree-lane-worker",
            "Never create or update a PR, decide merge readiness, merge",
        ),
        (
            "netguard-worktree-lane-worker",
            "Do not expand scope, independently infer\nmutation authority",
        ),
        ("git-safe-commit-push", "Hard stop when the current branch is `main`."),
    ],
)
def test_authority_guard_removal_is_rejected(tmp_path: Path, skill: str, guard: str) -> None:
    _copy_governance_surface(tmp_path)
    relative = validator.GOVERNANCE_SKILL_PATHS[skill]
    target = tmp_path / relative
    target.write_text(
        target.read_text(encoding="utf-8").replace(guard, "DRIFTED_AUTHORITY"), encoding="utf-8"
    )
    with pytest.raises(validator.ContractError, match="missing stable markers"):
        validator.validate_authority_boundaries(tmp_path)


@pytest.mark.parametrize(
    ("skill", "claim"),
    [
        ("github-pr-create-merge", "This skill independently decides merge readiness."),
        ("netguard-worktree-lane-worker", "This lane owns merge authority."),
        ("netguard-worktree-lane-worker", "This child grants mutation authority."),
        ("git-safe-commit-push", "Implementation commits on main are permitted."),
    ],
)
def test_contradictory_positive_authority_claim_is_rejected(
    tmp_path: Path, skill: str, claim: str
) -> None:
    _copy_governance_surface(tmp_path)
    relative = validator.GOVERNANCE_SKILL_PATHS[skill]
    target = tmp_path / relative
    target.write_text(f"{target.read_text(encoding='utf-8')}\n{claim}\n", encoding="utf-8")
    with pytest.raises(validator.ContractError, match="authority|protected-main"):
        validator.validate_authority_boundaries(tmp_path)


def test_multiagent_configuration_assignment_is_rejected(tmp_path: Path) -> None:
    _copy_governance_surface(tmp_path)
    (tmp_path / ".codex").mkdir(parents=True, exist_ok=True)
    (tmp_path / ".codex/config.toml").write_text("MultiAgentV2 = true\n", encoding="utf-8")
    (tmp_path / "AGENTS.md").write_text(
        (ROOT / "AGENTS.md").read_text(encoding="utf-8"), encoding="utf-8"
    )
    with pytest.raises(validator.ContractError, match="MultiAgentV2"):
        validator.validate_fallback_policy(tmp_path)
