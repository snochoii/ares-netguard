#!/usr/bin/env bash
set -euo pipefail

mode="${1:---tracked}"

case "${mode}" in
  --staged)
    mapfile -t files < <(git diff --cached --name-only)
    ;;
  --tracked)
    mapfile -t files < <(git ls-files)
    ;;
  *)
    echo "usage: $0 [--staged|--tracked]" >&2
    exit 2
    ;;
esac

is_allowed_fixture() {
  local path="$1"
  [[ "${path}" == tests/fixtures/* && "${path}" == *.jsonl ]]
}

is_forbidden_artifact() {
  local path="$1"

  if [[ "${path}" == ".env.example" ]]; then
    return 1
  fi

  case "${path}" in
    .venv/*|venv/*|env/*|.runtime/*|artifacts/*)
      return 0
      ;;
    .env|.env.*)
      return 0
      ;;
    data/pcap/*|data/zeek/*|data/suricata/*|data/falco/*)
      return 0
      ;;
    data/features/*|data/models/*|data/reports/*|data/registry/*)
      [[ "${path}" == */.gitkeep ]] && return 1
      return 0
      ;;
  esac

  case "${path}" in
    *.pcap|*.pcapng|*.parquet|*.joblib|*.pkl|*.onnx|*.pt|*.pth|*.ckpt|*.safetensors)
      return 0
      ;;
    *.db|*.sqlite|*.duckdb)
      return 0
      ;;
    *.jsonl)
      is_allowed_fixture "${path}" && return 1
      return 0
      ;;
  esac

  return 1
}

violations=()
for file in "${files[@]}"; do
  if is_forbidden_artifact "${file}"; then
    violations+=("${file}")
  fi
done

if ((${#violations[@]})); then
  echo "generated/private artifacts are not allowed:" >&2
  printf '  %s\n' "${violations[@]}" >&2
  exit 1
fi

echo "artifact guard clean: ${mode}"
