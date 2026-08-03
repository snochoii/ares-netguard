#!/usr/bin/env python3
"""Deterministic, offline drift checks for repository Codex workflow contracts."""

from __future__ import annotations

import argparse
import fnmatch
import re
import sys
import tomllib
from collections.abc import Iterable, Mapping, Sequence
from pathlib import Path, PurePosixPath
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
FIXTURE_PATH = ROOT / "tests/fixtures/codex_workflow/contracts.toml"

EXPECTED_AGENTS = frozenset(
    {
        "netguard-codebase-explorer",
        "netguard-correctness-reviewer",
        "netguard-docs-api-researcher",
        "netguard-integration-reviewer",
        "netguard-lane-worker",
        "netguard-product-security-reviewer",
    }
)
GOVERNANCE_SKILL_PATHS = {
    "netguard-orchestrator": ".agents/skills/netguard-orchestrator/SKILL.md",
    "netguard-parallel-dev": ".agents/skills/netguard-parallel-dev/SKILL.md",
    "netguard-worktree-lane-worker": (".agents/skills/netguard-worktree-lane-worker/SKILL.md"),
    "netguard-integration-merge": ".agents/skills/netguard-integration-merge/SKILL.md",
    "git-safe-commit-push": ".agents/skills/git-safe-commit-push/SKILL.md",
    "github-pr-create-merge": ".agents/skills/github-pr-create-merge/SKILL.md",
}
REFERENCE_POLICY_PATHS = (
    "docs/CODEX_ORCHESTRATOR_USAGE.md",
    "docs/MERGE_POLICY.md",
    "docs/TECHNOLOGY_SELECTION_POLICY.md",
)
REVIEW_CATEGORIES = frozenset(
    {"product_direction", "technology_choice", "architectural_tradeoff", "product_fit"}
)
REVIEW_PACKET_FIELDS = frozenset(
    {
        "review_category",
        "objective",
        "base_sha",
        "head_sha",
        "inspected_paths",
        "required_evidence",
        "decision_criteria",
        "output_contract",
        "stopping_conditions",
    }
)
LANE_PACKET_FIELDS = frozenset(
    {
        "skill_name",
        "skill_path",
        "objective",
        "base_sha",
        "worktree_path",
        "branch",
        "owned_paths",
        "forbidden_paths",
        "required_tests",
        "stopping_conditions",
        "commit_authority",
        "push_authority",
        "result_contract",
    }
)
READY_ACK_FIELDS = frozenset({"STATUS", "SKILL_ACK", "SKILL_PATH", "BASE_SHA", "CWD", "BRANCH"})
CAPABILITY_FAILURE_FIELDS = frozenset(
    {"STATUS", "CAPABILITY", "SKILL_NAME", "SKILL_PATH", "ROOT_ACTION"}
)
COMPLETION_FIELDS = frozenset(
    {
        "STATUS",
        "SKILL_ACK",
        "SKILL_PATH",
        "BASE_SHA",
        "HEAD_SHA",
        "CWD",
        "BRANCH",
        "CHANGED_PATHS",
        "FORBIDDEN_PATHS_TOUCHED",
        "TEST_RESULTS",
        "COMMIT_STATUS",
        "PUSH_STATUS",
        "UNRESOLVED_RISKS",
        "PARENT_ACTION",
    }
)
FALLBACK_ORDER = (
    "verified_named_agent",
    "verified_generic_child",
    "SIMULATED_ROOT_SERIAL",
)
SHARED_EXACT_PATHS = frozenset(
    {
        "AGENTS.md",
        ".codex/config.toml",
        *GOVERNANCE_SKILL_PATHS.values(),
        "Makefile",
        "pyproject.toml",
        "requirements-foundation-forecast.in",
        "requirements-foundation-forecast.lock",
        "scripts/check_no_generated_artifacts.sh",
        "scripts/validate_codex_workflow.py",
        "docs/MERGE_POLICY.md",
        "docs/TECHNOLOGY_SELECTION_POLICY.md",
        ".github/workflows/codex-workflow.yml",
    }
)
SHARED_PREFIX_PATHS = (".codex/agents",)
SHARED_GLOBS = ("requirements*.txt",)
SHA_RE = re.compile(r"[0-9a-fA-F]{40}\Z")
IDENTIFIER_RE = re.compile(r"\bnetguard-[a-z0-9-]+\b")


class ContractError(ValueError):
    """A deterministic workflow contract validation failure."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ContractError(message)


def read_text(root: Path, relative_path: str) -> str:
    path = root / relative_path
    try:
        return path.read_text(encoding="utf-8")
    except OSError as exc:
        raise ContractError(f"cannot read {relative_path}: {exc.strerror}") from exc


def load_toml(path: Path) -> dict[str, Any]:
    try:
        with path.open("rb") as handle:
            return tomllib.load(handle)
    except (OSError, tomllib.TOMLDecodeError) as exc:
        raise ContractError(f"invalid TOML {path}: {exc}") from exc


def section(text: str, heading: str) -> str:
    marker = f"## {heading}"
    lines = text.splitlines()
    try:
        start = lines.index(marker) + 1
    except ValueError as exc:
        raise ContractError(f"missing section: {marker}") from exc
    end = next(
        (index for index in range(start, len(lines)) if lines[index].startswith("## ")),
        len(lines),
    )
    return "\n".join(lines[start:end])


def require_markers(text: str, markers: Iterable[str], owner: str) -> None:
    normalized_text = " ".join(text.split())
    missing = [marker for marker in markers if " ".join(marker.split()) not in normalized_text]
    require(not missing, f"{owner}: missing stable markers: {missing}")


def normalize_sha(value: Any, field: str = "SHA") -> str:
    require(isinstance(value, str) and SHA_RE.fullmatch(value) is not None, f"{field}: invalid SHA")
    return value.lower()


def normalize_repo_path(value: Any, field: str = "path") -> str:
    require(isinstance(value, str) and value != "", f"{field}: expected non-empty path")
    require("\\" not in value, f"{field}: use repository-relative POSIX paths")
    path = PurePosixPath(value)
    require(not path.is_absolute(), f"{field}: absolute path is forbidden")
    require(".." not in path.parts, f"{field}: traversal is forbidden")
    normalized = path.as_posix()
    require(normalized not in {"", "."} and normalized == value, f"{field}: path is not normalized")
    return normalized


def validate_absolute_path(value: Any, field: str) -> str:
    require(isinstance(value, str) and value != "", f"{field}: expected non-empty path")
    require("\\" not in value, f"{field}: use POSIX path syntax")
    path = PurePosixPath(value)
    require(path.is_absolute(), f"{field}: path must be absolute")
    require(".." not in path.parts and path.as_posix() == value, f"{field}: path is not normalized")
    return value


def paths_overlap(left: str, right: str) -> bool:
    left_path = PurePosixPath(normalize_repo_path(left))
    right_path = PurePosixPath(normalize_repo_path(right))
    return (
        left_path == right_path
        or left_path in right_path.parents
        or right_path in left_path.parents
    )


def is_shared_chokepoint(value: str) -> bool:
    path = normalize_repo_path(value, "owned path")
    if any(paths_overlap(path, shared) for shared in SHARED_EXACT_PATHS):
        return True
    if any(paths_overlap(path, prefix) for prefix in SHARED_PREFIX_PATHS):
        return True
    return PurePosixPath(path).parent == PurePosixPath(".") and any(
        fnmatch.fnmatchcase(path, pattern) for pattern in SHARED_GLOBS
    )


def validate_shared_chokepoint_registry(root: Path) -> None:
    for path in sorted(SHARED_EXACT_PATHS):
        require((root / path).exists(), f"shared chokepoint does not exist: {path}")
        require(is_shared_chokepoint(path), f"shared chokepoint is not classified: {path}")
    require((root / ".codex/agents").is_dir(), "missing shared agent catalog")
    for path in sorted(root.glob("requirements*.txt")):
        relative = path.relative_to(root).as_posix()
        require(is_shared_chokepoint(relative), f"dependency entrypoint is not serial: {relative}")


def validate_branch(branch: Any) -> str:
    require(isinstance(branch, str) and branch != "", "branch: empty branch is forbidden")
    require(branch == branch.strip(), "branch: surrounding whitespace is forbidden")
    lowered = branch.lower()
    require(branch != "main", "branch: main is forbidden")
    require(
        lowered not in {"head", "detached", "(detached)"}
        and not lowered.startswith("(head detached"),
        "branch: detached HEAD is forbidden",
    )
    require(
        re.fullmatch(r"[A-Za-z0-9](?:[A-Za-z0-9._/-]*[A-Za-z0-9])?", branch) is not None,
        "branch: malformed branch name",
    )
    require(
        "//" not in branch and ".." not in branch and "@{" not in branch,
        "branch: malformed branch name",
    )
    require(
        all(part not in {"", ".", ".."} for part in branch.split("/")),
        "branch: malformed branch name",
    )
    return branch


def parse_frontmatter(text: str, owner: str) -> dict[str, str]:
    lines = text.splitlines()
    require(bool(lines) and lines[0] == "---", f"{owner}: missing frontmatter")
    try:
        end = lines.index("---", 1)
    except ValueError as exc:
        raise ContractError(f"{owner}: unterminated frontmatter") from exc
    values: dict[str, str] = {}
    for line in lines[1:end]:
        if not line.strip():
            continue
        match = re.fullmatch(r"([A-Za-z0-9_-]+):\s+(.+)", line)
        require(match is not None, f"{owner}: unsupported frontmatter line: {line!r}")
        key, value = match.groups()
        require(key not in values, f"{owner}: duplicate frontmatter key: {key}")
        values[key] = value.strip().strip('"').strip("'")
    require(bool(values.get("name")), f"{owner}: missing frontmatter name")
    require(bool(values.get("description")), f"{owner}: missing frontmatter description")
    return values


def load_skill_catalog(root: Path) -> dict[str, Path]:
    skill_root = root / ".agents/skills"
    require(skill_root.is_dir(), "missing .agents/skills")
    catalog: dict[str, Path] = {}
    for path in sorted(skill_root.glob("*/SKILL.md")):
        relative = path.relative_to(root).as_posix()
        frontmatter = parse_frontmatter(read_text(root, relative), relative)
        name = frontmatter["name"]
        require(name == path.parent.name, f"{relative}: directory/name mismatch")
        require(name not in catalog, f"duplicate skill name: {name}")
        catalog[name] = path
    missing = set(GOVERNANCE_SKILL_PATHS) - set(catalog)
    require(not missing, f"missing governance skills: {sorted(missing)}")
    return catalog


def load_agent_catalog(root: Path) -> dict[str, dict[str, Any]]:
    agent_root = root / ".codex/agents"
    require(agent_root.is_dir(), "missing .codex/agents")
    paths = sorted(agent_root.glob("*.toml"))
    require(len(paths) == 6, f"agent catalog: expected 6 TOML files, found {len(paths)}")
    catalog: dict[str, dict[str, Any]] = {}
    for path in paths:
        data = load_toml(path)
        relative = path.relative_to(root).as_posix()
        name = data.get("name")
        require(isinstance(name, str) and name == path.stem, f"{relative}: filename/name mismatch")
        require(name not in catalog, f"duplicate agent name: {name}")
        require(
            isinstance(data.get("description"), str) and data["description"],
            f"{relative}: missing description",
        )
        require(
            data.get("sandbox_mode") in {"read-only", "workspace-write"},
            f"{relative}: unsupported sandbox declaration",
        )
        instructions = data.get("developer_instructions")
        require(
            isinstance(instructions, str) and instructions.strip(),
            f"{relative}: missing instructions",
        )
        catalog[name] = data
    require(set(catalog) == EXPECTED_AGENTS, f"agent catalog mismatch: {sorted(catalog)}")
    sandboxes = [data["sandbox_mode"] for data in catalog.values()]
    require(sandboxes.count("read-only") == 5, "agent catalog: expected 5 read-only declarations")
    require(
        sandboxes.count("workspace-write") == 1,
        "agent catalog: expected 1 workspace-write declaration",
    )
    for name, data in catalog.items():
        instructions = data["developer_instructions"]
        if name == "netguard-lane-worker":
            require(
                data["sandbox_mode"] == "workspace-write",
                "lane worker must declare workspace-write",
            )
            require_markers(
                instructions,
                (
                    "assigned isolated worktree on a non-main branch",
                    "never forbidden_paths or shared chokepoints",
                    "explicitly authorizes each action",
                ),
                name,
            )
        else:
            require(data["sandbox_mode"] == "read-only", f"{name}: must declare read-only")
            require(
                instructions.lstrip().startswith("Read-only."),
                f"{name}: missing read-only instruction",
            )
            require(
                re.search(r"(?:Do not|Never) edit, stage, commit, push", instructions) is not None,
                f"{name}: missing mutation prohibition",
            )
        validate_agent_authority_text(name, instructions)
    require(
        "MERGE_READY" not in catalog["netguard-docs-api-researcher"]["developer_instructions"],
        "docs/API researcher must remain evidence-only",
    )
    return catalog


def validate_agent_authority_text(name: str, instructions: str) -> None:
    for forbidden in ("MERGE_GATE:", "MERGE_EXECUTION:"):
        require(
            forbidden not in instructions,
            f"{name}: unsupported authority marker {forbidden}",
        )
    positive_claims = (
        r"\b(?:may|can|allowed to|authorized to)\s+"
        r"(?:edit|stage|commit|push|merge|create|update|delete)\b",
        r"\b(?:owns?|decides?|grants?)\s+"
        r"(?:independent\s+)?(?:merge[- ]readiness|mutation authority)\b",
    )
    require(
        not any(re.search(pattern, instructions, re.IGNORECASE) for pattern in positive_claims),
        f"{name}: unsupported positive authority claim",
    )


def validate_root_contract(root: Path) -> None:
    path = root / "AGENTS.md"
    require(path.is_file(), "missing AGENTS.md")
    raw = path.read_bytes()
    require(len(raw) < 16_384, "AGENTS.md exceeds 16,384-byte limit")
    text = raw.decode("utf-8")
    authority = section(text, "Instruction and authority model")
    require_markers(
        authority,
        (
            "Routing and mutation authority are separate:",
            "Plan, audit, explain, review, and research requests are read-only",
        ),
        "AGENTS.md authority",
    )
    isolation = section(text, "Branch and write isolation")
    require_markers(
        isolation,
        (
            "Normal implementation commits directly on `main` are forbidden.",
            "dedicated non-main branch",
            "Shared chokepoints execute serially",
        ),
        "AGENTS.md branch isolation",
    )
    delegation = section(text, "Delegation contract")
    ordered = (
        "1. A verified named custom agent.",
        "2. A generic child with a complete task packet.",
        "3. Root-thread serial execution of the same packet.",
    )
    positions = [delegation.find(marker) for marker in ordered]
    require(
        all(position >= 0 for position in positions) and positions == sorted(positions),
        "AGENTS.md: fallback order drift",
    )
    require_markers(
        delegation,
        (
            '`fork_turns: "none"`',
            "A batch must not fail only because named agents are unavailable.",
            '`sandbox_mode = "read-only"` in an agent TOML is a declaration, not proof',
            "perform it serially in the root thread",
            "Do not enable\n`MultiAgentV2` manually.",
        ),
        "AGENTS.md delegation",
    )
    review = section(text, "Review and merge contract")
    require_markers(
        review,
        (
            "Every merge-gating reviewer must bind its result to the exact reviewed head SHA.",
            "MERGE_READY: yes",
            "MERGE_READY: no",
            "HEAD_SHA: <reviewed_head_sha>",
            "Any head change invalidates all earlier review results",
        ),
        "AGENTS.md review",
    )


def validate_policy_references(root: Path) -> None:
    text = read_text(root, "AGENTS.md")
    policy = section(text, "Product and technology boundaries")
    references = re.findall(r"^- `(docs/[A-Za-z0-9_./-]+\.md)`$", policy, re.MULTILINE)
    require(bool(references), "AGENTS.md: no canonical policy references found")
    missing = [reference for reference in references if not (root / reference).is_file()]
    require(not missing, f"AGENTS.md: missing canonical policy paths: {missing}")


def workflow_reference_paths(root: Path) -> list[Path]:
    fixed = [root / "AGENTS.md", *(root / path for path in REFERENCE_POLICY_PATHS)]
    return [
        *fixed,
        *sorted((root / ".codex/agents").glob("*.toml")),
        *sorted((root / ".agents/skills").glob("*/SKILL.md")),
    ]


def validate_identifier_references(
    references: Iterable[str], valid_identifiers: set[str], allowlist: Iterable[str] = ()
) -> None:
    allowed = set(allowlist)
    unknown = set(references) - valid_identifiers - allowed
    require(not unknown, f"dangling netguard identifiers: {sorted(unknown)}")


def validate_repository_references(
    root: Path,
    agent_catalog: Mapping[str, Any],
    skill_catalog: Mapping[str, Any],
    allowlist: Iterable[str] = (),
) -> None:
    references: set[str] = set()
    for path in workflow_reference_paths(root):
        require(path.is_file(), f"missing reference surface: {path.relative_to(root)}")
        references.update(IDENTIFIER_RE.findall(path.read_text(encoding="utf-8")))
    validate_identifier_references(references, set(agent_catalog) | set(skill_catalog), allowlist)


def validate_authority_boundaries(root: Path) -> None:
    markers = {
        "netguard-orchestrator": (
            "Own user-goal interpretation, mutation-authority checks, preflight",
            "execution-path selection",
            "final root judgment",
            "execute the unchanged packet root-serial",
        ),
        "netguard-parallel-dev": (
            "Own parallel eligibility, shared-chokepoint classification, and lane topology.",
            "Do not implement, commit, push, review, create or merge PRs",
        ),
        "netguard-worktree-lane-worker": (
            "Own the delegated implementation packet, pre-edit acknowledgment, bounded lane",
            "Do not expand scope, independently infer\nmutation authority",
            "Never create or update a PR, decide merge readiness, merge",
        ),
        "netguard-integration-merge": (
            "the sole merge-readiness decision",
            "Do not grant mutation authority, operate GitHub merge transport",
            "MERGE_GATE: ready | blocked",
        ),
        "git-safe-commit-push": (
            "Own only an explicitly authorized local commit and an independently authorized",
            "optional push",
            "Hard stop when the current branch is `main`.",
            "Do not create branches, create or update PRs, merge",
        ),
        "github-pr-create-merge": (
            "Own GitHub PR create, update, read, checks lookup, and an already-authorized",
            "merge execution",
            "Do not route reviews, parse review receipts, decide readiness,",
            "Do not reinterpret validation or reviews and do not independently approve the",
            "MERGE_EXECUTION: authorized",
        ),
    }
    for name, expected in markers.items():
        text = read_text(root, GOVERNANCE_SKILL_PATHS[name])
        require_markers(text, expected, name)
        reject_governance_authority_claims(name, text)


def reject_governance_authority_claims(name: str, text: str) -> None:
    sentences = re.split(r"(?<=[.!?])\s+", " ".join(text.split()))
    negative_cues = ("do not", "never", "without", "does not", "cannot", "forbidden")
    for sentence in sentences:
        lowered = sentence.lower()
        if any(cue in lowered for cue in negative_cues):
            continue
        if name == "github-pr-create-merge" and re.search(
            r"\b(?:owns?|decides?|grants?|approves?)\b.*"
            r"\b(?:merge[- ]readiness|readiness decision|merge approval)\b",
            lowered,
        ):
            raise ContractError(f"{name}: positive merge-readiness authority claim")
        if name == "netguard-worktree-lane-worker" and re.search(
            r"\b(?:owns?|decides?|grants?|authorizes?)\b.*"
            r"\b(?:merge authority|merge readiness|mutation authority)\b",
            lowered,
        ):
            raise ContractError(f"{name}: positive child authority claim")
        if name == "git-safe-commit-push" and re.search(
            r"\bmain\b.*\b(?:allowed|authorized|permitted)\b", lowered
        ):
            raise ContractError(f"{name}: protected-main rule weakened")


def parse_review_receipt(text: Any, current_head: str) -> dict[str, Any]:
    expected_head = normalize_sha(current_head, "current HEAD")
    require(isinstance(text, str) and text != "", "receipt: missing output")
    lines = text.splitlines()
    require(len(lines) >= 2, "receipt: missing two-line header")
    require(lines[0] in {"MERGE_READY: yes", "MERGE_READY: no"}, "receipt: malformed first line")
    require(
        re.fullmatch(r"HEAD_SHA: [0-9a-fA-F]{40}", lines[1]) is not None,
        "receipt: malformed second line",
    )
    reviewed_head = normalize_sha(lines[1].removeprefix("HEAD_SHA: "), "receipt HEAD_SHA")
    require(reviewed_head == expected_head, "receipt: stale HEAD_SHA")
    for line in lines[2:]:
        require(
            not line.startswith("MERGE_READY:") and not line.startswith("HEAD_SHA:"),
            "receipt: duplicated header marker",
        )
    return {"ready": lines[0] == "MERGE_READY: yes", "head_sha": reviewed_head}


def require_exact_fields(data: Mapping[str, Any], fields: frozenset[str], owner: str) -> None:
    actual = set(data)
    missing = sorted(fields - actual)
    unexpected = sorted(actual - fields)
    require(
        not missing and not unexpected, f"{owner}: fields missing={missing} unexpected={unexpected}"
    )


def nonempty_string(value: Any, field: str) -> str:
    require(isinstance(value, str) and bool(value.strip()), f"{field}: expected non-empty string")
    return value


def string_list(value: Any, field: str, *, nonempty: bool = True) -> list[str]:
    require(isinstance(value, list), f"{field}: expected list")
    if nonempty:
        require(bool(value), f"{field}: expected non-empty list")
    require(
        all(isinstance(item, str) and item.strip() for item in value), f"{field}: invalid list item"
    )
    return value


def validate_review_packet(packet: Mapping[str, Any]) -> None:
    require(isinstance(packet, Mapping), "review packet: expected table")
    require_exact_fields(packet, REVIEW_PACKET_FIELDS, "review packet")
    require(packet["review_category"] in REVIEW_CATEGORIES, "review packet: invalid category")
    nonempty_string(packet["objective"], "review packet objective")
    normalize_sha(packet["base_sha"], "review packet base_sha")
    normalize_sha(packet["head_sha"], "review packet head_sha")
    for path in string_list(packet["inspected_paths"], "review packet inspected_paths"):
        normalize_repo_path(path, "review packet inspected path")
    string_list(packet["required_evidence"], "review packet required_evidence")
    string_list(packet["decision_criteria"], "review packet decision_criteria")
    require(
        packet["output_contract"] == "sha_bound_review_v1", "review packet: invalid output contract"
    )
    string_list(packet["stopping_conditions"], "review packet stopping_conditions")


def validate_lane_packet(packet: Mapping[str, Any], root: Path = ROOT) -> None:
    require(isinstance(packet, Mapping), "lane packet: expected table")
    require_exact_fields(packet, LANE_PACKET_FIELDS, "lane packet")
    skill_name = nonempty_string(packet["skill_name"], "lane packet skill_name")
    skill_path = normalize_repo_path(packet["skill_path"], "lane packet skill_path")
    require(
        skill_path.startswith(".agents/skills/") and skill_path.endswith("/SKILL.md"),
        "lane packet: invalid skill path",
    )
    full_skill_path = root / skill_path
    require(full_skill_path.is_file(), "lane packet: skill path does not exist")
    frontmatter = parse_frontmatter(full_skill_path.read_text(encoding="utf-8"), skill_path)
    require(frontmatter["name"] == skill_name, "lane packet: skill name/path mismatch")
    nonempty_string(packet["objective"], "lane packet objective")
    normalize_sha(packet["base_sha"], "lane packet base_sha")
    validate_absolute_path(packet["worktree_path"], "lane packet worktree_path")
    validate_branch(packet["branch"])
    owned = [
        normalize_repo_path(path, "lane owned path")
        for path in string_list(packet["owned_paths"], "lane owned_paths")
    ]
    forbidden = [
        normalize_repo_path(path, "lane forbidden path")
        for path in string_list(packet["forbidden_paths"], "lane forbidden_paths", nonempty=False)
    ]
    require(
        not any(paths_overlap(left, right) for left in owned for right in forbidden),
        "lane packet: owned/forbidden overlap",
    )
    require(
        not any(is_shared_chokepoint(path) for path in owned),
        "lane packet: shared chokepoint owned",
    )
    string_list(packet["required_tests"], "lane packet required_tests")
    string_list(packet["stopping_conditions"], "lane packet stopping_conditions")
    require(
        packet["commit_authority"] in {"authorized", "not_authorized"},
        "lane packet: invalid commit authority",
    )
    require(
        packet["push_authority"] in {"authorized", "not_authorized"},
        "lane packet: invalid push authority",
    )
    require(
        not (
            packet["push_authority"] == "authorized" and packet["commit_authority"] != "authorized"
        ),
        "lane packet: push cannot be authorized without commit",
    )
    require(packet["result_contract"] == "lane_result_v1", "lane packet: invalid result contract")


def validate_fallback_packet(original: Mapping[str, Any], fallback: Mapping[str, Any]) -> None:
    require(dict(original) == dict(fallback), "fallback must reuse the unchanged packet")


def validate_lane_set(packets: Sequence[Mapping[str, Any]], root: Path = ROOT) -> None:
    for packet in packets:
        validate_lane_packet(packet, root)
    for index, left in enumerate(packets):
        for right in packets[index + 1 :]:
            require(
                left["branch"] != right["branch"],
                "lane set: writer branches must be distinct",
            )
            require(
                Path(left["worktree_path"]) != Path(right["worktree_path"]),
                "lane set: writer worktrees must be distinct",
            )
            require(
                not any(
                    paths_overlap(left_path, right_path)
                    for left_path in left["owned_paths"]
                    for right_path in right["owned_paths"]
                ),
                "lane set: cross-lane owned-path overlap",
            )


def parse_field_block(text: Any, fields: frozenset[str], owner: str) -> dict[str, str]:
    require(isinstance(text, str) and text != "", f"{owner}: missing output")
    require(text == text.strip("\n"), f"{owner}: leading/trailing blank line")
    values: dict[str, str] = {}
    for line in text.splitlines():
        match = re.fullmatch(r"([A-Z_]+): (.+)", line)
        require(match is not None, f"{owner}: malformed line {line!r}")
        key, value = match.groups()
        require(value == value.strip(), f"{owner}: malformed value for {key}")
        require(key not in values, f"{owner}: duplicate field {key}")
        values[key] = value
    require_exact_fields(values, fields, owner)
    return values


def validate_ready_ack(text: Any, packet: Mapping[str, Any] | None = None) -> dict[str, str]:
    values = parse_field_block(text, READY_ACK_FIELDS, "ready acknowledgment")
    require(values["STATUS"] == "ready", "ready acknowledgment: invalid status")
    normalize_sha(values["BASE_SHA"], "ready acknowledgment BASE_SHA")
    normalize_repo_path(values["SKILL_PATH"], "ready acknowledgment SKILL_PATH")
    validate_absolute_path(values["CWD"], "ready acknowledgment CWD")
    validate_branch(values["BRANCH"])
    if packet is not None:
        expected = {
            "SKILL_ACK": packet["skill_name"],
            "SKILL_PATH": packet["skill_path"],
            "BASE_SHA": str(packet["base_sha"]),
            "CWD": packet["worktree_path"],
            "BRANCH": packet["branch"],
        }
        for key, value in expected.items():
            require(
                values[key].lower() == value.lower() if key == "BASE_SHA" else values[key] == value,
                f"ready acknowledgment: {key} mismatch",
            )
    return values


def validate_capability_failure(
    text: Any, packet: Mapping[str, Any] | None = None
) -> dict[str, str]:
    values = parse_field_block(text, CAPABILITY_FAILURE_FIELDS, "capability failure")
    require(values["STATUS"] == "capability_failure", "capability failure: invalid status")
    require(values["CAPABILITY"] == "required_skill", "capability failure: invalid capability")
    normalize_repo_path(values["SKILL_PATH"], "capability failure SKILL_PATH")
    require(
        values["ROOT_ACTION"] == "execute_same_packet_serially",
        "capability failure: invalid ROOT_ACTION",
    )
    if packet is not None:
        require(
            values["SKILL_NAME"] == packet["skill_name"],
            "capability failure: SKILL_NAME mismatch",
        )
        require(
            values["SKILL_PATH"] == packet["skill_path"],
            "capability failure: SKILL_PATH mismatch",
        )
    return values


def parse_reported_paths(value: str, owner: str) -> list[str]:
    if value == "none":
        return []
    paths = value.split(", ")
    require(
        ", ".join(paths) == value and all(paths),
        f"{owner}: expected 'none' or a comma-space-separated path list",
    )
    return [normalize_repo_path(path, owner) for path in paths]


def path_is_within(path: str, owner: str) -> bool:
    return path == owner or path.startswith(f"{owner}/")


def validate_completion_result(
    text: Any, packet: Mapping[str, Any] | None = None
) -> dict[str, str]:
    values = parse_field_block(text, COMPLETION_FIELDS, "completion result")
    require(
        values["STATUS"] in {"completed", "blocked", "capability_failure"},
        "completion result: invalid status",
    )
    normalize_repo_path(values["SKILL_PATH"], "completion result SKILL_PATH")
    normalize_sha(values["BASE_SHA"], "completion result BASE_SHA")
    normalize_sha(values["HEAD_SHA"], "completion result HEAD_SHA")
    validate_absolute_path(values["CWD"], "completion result CWD")
    validate_branch(values["BRANCH"])
    expected_actions = {
        "completed": "integrate",
        "blocked": "inspect_blocker",
        "capability_failure": "execute_same_packet_serially",
    }
    require(
        values["PARENT_ACTION"] == expected_actions[values["STATUS"]],
        "completion result: status/PARENT_ACTION mismatch",
    )
    changed_paths = parse_reported_paths(values["CHANGED_PATHS"], "completion result CHANGED_PATHS")
    forbidden_touched = parse_reported_paths(
        values["FORBIDDEN_PATHS_TOUCHED"],
        "completion result FORBIDDEN_PATHS_TOUCHED",
    )
    commit_status = values["COMMIT_STATUS"]
    push_status = values["PUSH_STATUS"]
    if commit_status not in {"not_authorized", "not_created"}:
        normalize_sha(commit_status, "completion result COMMIT_STATUS")
    nonempty_string(push_status, "completion result PUSH_STATUS")
    if packet is not None:
        for key, packet_key in (
            ("SKILL_ACK", "skill_name"),
            ("SKILL_PATH", "skill_path"),
            ("BASE_SHA", "base_sha"),
            ("CWD", "worktree_path"),
            ("BRANCH", "branch"),
        ):
            actual = values[key]
            expected = str(packet[packet_key])
            require(
                actual.lower() == expected.lower() if key == "BASE_SHA" else actual == expected,
                f"completion result: {key} mismatch",
            )
        owned_paths = [str(path) for path in packet["owned_paths"]]
        require(
            all(
                any(path_is_within(path, owned) for owned in owned_paths) for path in changed_paths
            ),
            "completion result: changed path outside owned paths",
        )
        require(
            not any(
                paths_overlap(path, forbidden)
                for path in changed_paths
                for forbidden in packet["forbidden_paths"]
            ),
            "completion result: changed path overlaps forbidden paths",
        )
        if packet["commit_authority"] == "not_authorized":
            require(
                commit_status == "not_authorized",
                "completion result: unauthorized commit status",
            )
        if packet["push_authority"] == "not_authorized":
            require(
                push_status == "not_authorized",
                "completion result: unauthorized push status",
            )
    if values["PARENT_ACTION"] == "integrate":
        require(
            not forbidden_touched,
            "completion result: integration blocked by forbidden paths",
        )
        require(
            re.search(r"\bpassed\b", values["TEST_RESULTS"], re.IGNORECASE) is not None
            and re.search(r"\b(failed|error|not[_ ]run)\b", values["TEST_RESULTS"], re.IGNORECASE)
            is None,
            "completion result: integration requires passing tests",
        )
        require(
            values["UNRESOLVED_RISKS"] == "none",
            "completion result: integration blocked by unresolved risks",
        )
    return values


def validate_path_ownership(lanes: Sequence[Mapping[str, Any]]) -> None:
    writers: list[tuple[str, list[str]]] = []
    for lane in lanes:
        name = nonempty_string(lane.get("name"), "path ownership lane name")
        kind = lane.get("kind")
        require(kind in {"writer", "read-only"}, f"path ownership {name}: invalid lane kind")
        owned = [
            normalize_repo_path(path, f"path ownership {name} owned path")
            for path in string_list(
                lane.get("owned_paths"), f"path ownership {name} owned_paths", nonempty=False
            )
        ]
        forbidden = [
            normalize_repo_path(path, f"path ownership {name} forbidden path")
            for path in string_list(
                lane.get("forbidden_paths", []),
                f"path ownership {name} forbidden_paths",
                nonempty=False,
            )
        ]
        if kind == "read-only":
            continue
        require(bool(owned), f"path ownership {name}: writer needs owned paths")
        require(
            not any(paths_overlap(left, right) for left in owned for right in forbidden),
            f"path ownership {name}: owned/forbidden overlap",
        )
        require(
            not any(is_shared_chokepoint(path) for path in owned),
            f"path ownership {name}: shared chokepoint owned",
        )
        writers.append((name, owned))
    for index, (left_name, left_paths) in enumerate(writers):
        for right_name, right_paths in writers[index + 1 :]:
            require(
                not any(paths_overlap(left, right) for left in left_paths for right in right_paths),
                f"path ownership: writer overlap {left_name}/{right_name}",
            )


def validate_fallback_order(order: Sequence[str]) -> None:
    require(tuple(order) == FALLBACK_ORDER, "fallback order mismatch")


def validate_fallback_policy(root: Path) -> None:
    orchestrator = read_text(root, GOVERNANCE_SKILL_PATHS["netguard-orchestrator"])
    root_contract = read_text(root, "AGENTS.md")
    steps = (
        "1. Use a verified named custom agent",
        "2. If named selection is unavailable",
        "3. If spawning, skill loading, isolation, or permission verification fails",
    )
    positions = [orchestrator.find(step) for step in steps]
    require(
        all(position >= 0 for position in positions) and positions == sorted(positions),
        "orchestrator: fallback order drift",
    )
    require_markers(
        orchestrator,
        (
            '`fork_turns: "none"`',
            "Never enable `MultiAgentV2` manually.",
            "Trust the surface only when the write is denied",
            "If the write succeeds or enforcement is ambiguous",
            "discard the probe and\n   every result from that child",
            "verified CLI `--sandbox read-only`",
            "EXECUTION_MODE: SIMULATED_ROOT_SERIAL",
            "ACTUAL_SPAWN_COUNT_DELTA: 0",
            "ACTUAL_HANDOFF_COUNT_DELTA: 0",
        ),
        "orchestrator fallback",
    )
    require_markers(
        root_contract,
        (
            "A batch must not fail only because named agents are unavailable.",
            "Do not enable `MultiAgentV2` manually.",
        ),
        "root fallback",
    )
    config = read_text(root, ".codex/config.toml")
    require("MultiAgentV2" not in config, ".codex/config.toml manually configures MultiAgentV2")
    for path in GOVERNANCE_SKILL_PATHS.values():
        text = read_text(root, path)
        require(
            re.search(r"MultiAgentV2\s*=|enable\s+MultiAgentV2", text, re.IGNORECASE) is None,
            f"{path}: manually enables MultiAgentV2",
        )


def validate_merge_ownership(root: Path) -> None:
    integration = read_text(root, GOVERNANCE_SKILL_PATHS["netguard-integration-merge"])
    github = read_text(root, GOVERNANCE_SKILL_PATHS["github-pr-create-merge"])
    orchestrator = read_text(root, GOVERNANCE_SKILL_PATHS["netguard-orchestrator"])
    require_markers(
        integration,
        (
            "Accept a review only from an execution surface whose effective read-only",
            "MERGE_GATE: ready | blocked",
            "Invalidate all receipts and every earlier readiness result "
            "after any candidate-head change.",
            "missing,\nmalformed, negative, duplicated, or extra-required receipt as blocking",
        ),
        "integration merge ownership",
    )
    require_markers(
        orchestrator,
        (
            "issue the exact\n   `MERGE_EXECUTION` authorization",
            "If the gate is ready and merge authority exists",
        ),
        "root merge ownership",
    )
    require_markers(
        github,
        (
            "# GitHub PR Transport",
            "MERGE_EXECUTION: authorized",
            "remote PR head differs from `HEAD_SHA`",
            "do not independently approve the\nmerge",
        ),
        "GitHub transport ownership",
    )


def validate_entrypoints(root: Path) -> None:
    makefile = read_text(root, "Makefile")
    require(
        (root / "scripts/check_no_generated_artifacts.sh").is_file(),
        "missing generated-artifact guard",
    )
    require_markers(
        makefile,
        (
            "PYTHON ?= $(shell if [ -x .venv/bin/python ]; then echo .venv/bin/python; "
            "else echo python3; fi)",
            ".PHONY: verify verify-codex-workflow",
            "verify: verify-codex-workflow",
            "verify-codex-workflow:",
            "$(PYTHON) scripts/validate_codex_workflow.py all",
            "$(PYTHON) -m pytest -q tests/unit/test_codex_workflow_contract.py",
            "bash scripts/check_no_generated_artifacts.sh --tracked",
            "bash scripts/check_no_generated_artifacts.sh --staged",
        ),
        "Makefile Codex workflow entrypoint",
    )

    project = load_toml(root / "pyproject.toml")
    requires_python = project.get("project", {}).get("requires-python")
    minimum = re.fullmatch(r">=(\d+)\.(\d+)", str(requires_python))
    require(minimum is not None, "pyproject.toml: unsupported requires-python shape")
    minimum_version = tuple(int(part) for part in minimum.groups())
    dev_dependencies = project.get("project", {}).get("optional-dependencies", {}).get("dev", [])
    pytest_spec = next(
        (dependency for dependency in dev_dependencies if dependency.startswith("pytest")), None
    )
    require(pytest_spec is not None, "pyproject.toml: missing pytest dev dependency")

    workflow = read_text(root, ".github/workflows/codex-workflow.yml")
    require_markers(
        workflow,
        (
            "pull_request:",
            "push:",
            "branches:",
            "- main",
            "permissions:",
            "contents: read",
            "persist-credentials: false",
            "python3 -m pytest --version",
            "PYTHON=python3 make verify-codex-workflow",
        ),
        "Codex workflow CI",
    )
    require("pull_request_target:" not in workflow, "Codex workflow CI: unsafe PR trigger")
    permission_blocks = re.findall(r"^permissions:\n((?:  [^\n]+\n)+)", workflow, re.MULTILINE)
    require(
        len(permission_blocks) == 1
        and permission_blocks[0].strip().splitlines() == ["contents: read"],
        "Codex workflow CI: permissions must be exactly contents: read",
    )
    actions = re.findall(r"^\s*uses:\s+([^@\s]+)@([^\s#]+)", workflow, re.MULTILINE)
    require(
        [name for name, _ in actions] == ["actions/checkout", "actions/setup-python"],
        "Codex workflow CI: unexpected action set",
    )
    require(
        all(re.fullmatch(r"[0-9a-f]{40}", ref) is not None for _, ref in actions),
        "Codex workflow CI: actions must use immutable lowercase commit pins",
    )
    configured_python = re.search(
        r'^\s*python-version:\s*["\']?(\d+)\.(\d+)["\']?\s*$', workflow, re.MULTILINE
    )
    require(configured_python is not None, "Codex workflow CI: missing Python version")
    ci_version = tuple(int(part) for part in configured_python.groups())
    require(ci_version >= minimum_version, "Codex workflow CI: Python is below supported range")
    expected_range_check = f"sys.version_info >= {minimum_version!r}"
    require(expected_range_check in workflow, "Codex workflow CI: Python range check drift")
    require(pytest_spec in workflow, "Codex workflow CI: focused pytest dependency drift")
    require(".venv" not in workflow, "Codex workflow CI: repository .venv dependency is forbidden")
    require("secrets." not in workflow, "Codex workflow CI: secrets are forbidden")
    run_commands = re.findall(r"^\s*run:\s*(.+)$", workflow, re.MULTILINE)
    require(
        run_commands
        == [
            f"python3 -c 'import sys; assert sys.version_info >= {minimum_version!r}, sys.version'",
            f'python3 -m pip install --disable-pip-version-check "{pytest_spec}"',
            "python3 -m pytest --version",
            "PYTHON=python3 make verify-codex-workflow",
        ],
        "Codex workflow CI: unexpected run command set",
    )
    forbidden_calls = re.compile(r"(?:^|\s)(?:codex|gh|curl|wget)(?:\s|$)")
    require(
        not any(forbidden_calls.search(command) for command in run_commands),
        "Codex workflow CI: live Codex/GitHub/external-service call found",
    )


def validate_repo(root: Path = ROOT) -> None:
    validate_root_contract(root)
    validate_policy_references(root)
    agents = load_agent_catalog(root)
    skills = load_skill_catalog(root)
    validate_repository_references(root, agents, skills)
    validate_authority_boundaries(root)
    validate_fallback_policy(root)
    validate_merge_ownership(root)
    validate_entrypoints(root)
    validate_shared_chokepoint_registry(root)


def expect_case(case: Mapping[str, Any], callback: Any) -> None:
    expected = case.get("valid")
    require(isinstance(expected, bool), f"fixture {case.get('name')}: missing valid flag")
    try:
        callback()
    except ContractError:
        if expected:
            raise
    else:
        require(expected, f"fixture {case.get('name')}: expected rejection")


def validate_fixtures(root: Path = ROOT) -> None:
    fixture_path = root / "tests/fixtures/codex_workflow/contracts.toml"
    data = load_toml(fixture_path)
    require(
        data.get("meta", {}).get("schema") == "codex_workflow_contract_fixtures_v1",
        "fixture schema mismatch",
    )

    for case in data.get("branch_cases", []):
        expect_case(case, lambda case=case: validate_branch(case["branch"]))

    for case in data.get("receipt_cases", []):

        def receipt_check(case: Mapping[str, Any] = case) -> None:
            parsed = parse_review_receipt(case["text"], case["current_head"])
            if "ready" in case:
                require(
                    parsed["ready"] is case["ready"], f"fixture {case['name']}: readiness mismatch"
                )

        expect_case(case, receipt_check)

    for case in data.get("review_packet_cases", []):
        expect_case(case, lambda case=case: validate_review_packet(case["packet"]))

    lane_cases = {case["name"]: case for case in data.get("lane_packet_cases", [])}
    for case in lane_cases.values():
        expect_case(case, lambda case=case: validate_lane_packet(case["packet"], root))

    for case in data.get("fallback_packet_cases", []):
        expect_case(
            case,
            lambda case=case: validate_fallback_packet(
                lane_cases[case["original"]]["packet"], lane_cases[case["fallback"]]["packet"]
            ),
        )

    for case in data.get("lane_set_cases", []):
        expect_case(
            case,
            lambda case=case: validate_lane_set(
                [lane_cases[name]["packet"] for name in case["packets"]], root
            ),
        )

    for table_name, callback in (
        ("ready_ack_cases", validate_ready_ack),
        ("capability_failure_cases", validate_capability_failure),
        ("completion_result_cases", validate_completion_result),
    ):
        for case in data.get(table_name, []):
            expect_case(
                case,
                lambda case=case, callback=callback: callback(
                    case["text"], lane_cases[case["packet"]]["packet"]
                ),
            )

    for case in data.get("path_ownership_cases", []):
        expect_case(case, lambda case=case: validate_path_ownership(case["lanes"]))

    for case in data.get("identifier_cases", []):
        expect_case(
            case,
            lambda case=case: validate_identifier_references(
                case["references"], set(case["valid_identifiers"]), case.get("allowlist", [])
            ),
        )

    for case in data.get("fallback_order_cases", []):
        expect_case(case, lambda case=case: validate_fallback_order(case["order"]))


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("repo", "fixtures", "all"))
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    try:
        if args.mode in {"repo", "all"}:
            validate_repo(ROOT)
        if args.mode in {"fixtures", "all"}:
            validate_fixtures(ROOT)
    except ContractError as exc:
        print(f"codex-workflow: error: {exc}", file=sys.stderr)
        return 1
    suffix = "static sandbox declarations only; runtime enforcement not tested"
    print(f"codex-workflow: {args.mode} valid ({suffix})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
