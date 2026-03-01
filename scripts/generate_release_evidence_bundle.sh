#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage:
  ./scripts/generate_release_evidence_bundle.sh --tag <tag> [options]

Options:
  --tag <value>               Release tag (required).
  --gate <value>              Gate identifier (e.g. G26). If omitted, inferred from tag when possible.
  --status <value>            Release status. Default: closed.
  --scope <value>             Release scope summary.
  --sprint-report <path>      Sprint report path to extract verification lines.
  --progress-snapshot <path>  Progress snapshot path. Default: sprints/reports/PROGRESS_SNAPSHOT.md.
  --commit-range <range>      Git range for key commits (e.g. v1.0..HEAD).
  --max-commits <n>           Maximum commits to include. Default: 20.
  --output-dir <path>         Output directory. Default: sprints/reports/releases/<tag>.
  --help                      Show this message.
USAGE
}

TAG=""
GATE=""
STATUS="closed"
SCOPE=""
SPRINT_REPORT=""
PROGRESS_SNAPSHOT="sprints/reports/PROGRESS_SNAPSHOT.md"
COMMIT_RANGE=""
MAX_COMMITS=20
OUTPUT_DIR=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --tag)
      TAG="${2:-}"
      shift 2
      ;;
    --gate)
      GATE="${2:-}"
      shift 2
      ;;
    --status)
      STATUS="${2:-}"
      shift 2
      ;;
    --scope)
      SCOPE="${2:-}"
      shift 2
      ;;
    --sprint-report)
      SPRINT_REPORT="${2:-}"
      shift 2
      ;;
    --progress-snapshot)
      PROGRESS_SNAPSHOT="${2:-}"
      shift 2
      ;;
    --commit-range)
      COMMIT_RANGE="${2:-}"
      shift 2
      ;;
    --max-commits)
      MAX_COMMITS="${2:-}"
      shift 2
      ;;
    --output-dir)
      OUTPUT_DIR="${2:-}"
      shift 2
      ;;
    --help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1"
      usage
      exit 1
      ;;
  esac
done

if [[ -z "$TAG" ]]; then
  echo "ERROR: --tag is required."
  usage
  exit 1
fi

if [[ -z "$GATE" ]]; then
  if [[ "$TAG" =~ [gG]([0-9]{2}) ]]; then
    GATE="G${BASH_REMATCH[1]}"
  else
    GATE="UNSPECIFIED"
  fi
fi

if [[ -z "$SCOPE" ]]; then
  SCOPE="Automated close-out evidence package."
fi

if [[ -z "$OUTPUT_DIR" ]]; then
  OUTPUT_DIR="sprints/reports/releases/${TAG}"
fi

if [[ -z "$SPRINT_REPORT" ]]; then
  if [[ -f "sprints/active.toml" ]]; then
    active_sprint="$(
      awk -F'=' '/^active_sprint/{gsub(/ /,"",$2); print $2}' sprints/active.toml
    )"
    if [[ -n "$active_sprint" ]]; then
      SPRINT_REPORT="$(printf "sprints/reports/SPRINT-%03d.md" "$active_sprint")"
    fi
  fi
fi

if [[ -z "$COMMIT_RANGE" ]]; then
  previous_tag="$(git tag --sort=-creatordate | grep -v "^${TAG}$" | head -n 1 || true)"
  if [[ -n "$previous_tag" ]]; then
    COMMIT_RANGE="${previous_tag}..HEAD"
  fi
fi

if [[ -n "$COMMIT_RANGE" ]]; then
  mapfile -t commit_lines < <(git log --oneline --max-count "$MAX_COMMITS" "$COMMIT_RANGE")
else
  mapfile -t commit_lines < <(git log --oneline --max-count "$MAX_COMMITS")
fi

if [[ ${#commit_lines[@]} -eq 0 ]]; then
  mapfile -t commit_lines < <(git log --oneline --max-count 1)
fi

commit_ids=()
for line in "${commit_lines[@]}"; do
  commit_ids+=("${line%% *}")
done

verification_lines=()
if [[ -n "$SPRINT_REPORT" && -f "$SPRINT_REPORT" ]]; then
  while IFS= read -r line; do
    if [[ "$line" =~ ^-[[:space:]] ]]; then
      verification_lines+=("$line")
    fi
  done < <(awk '/^## Verification Summary/{flag=1; next} /^## /{if(flag) exit} flag{print}' "$SPRINT_REPORT")
fi

mkdir -p "$OUTPUT_DIR"

release_notes_file="${OUTPUT_DIR}/RELEASE_NOTES.md"
evidence_bundle_file="${OUTPUT_DIR}/EVIDENCE_BUNDLE.md"
manifest_file="${OUTPUT_DIR}/MANIFEST.yaml"

today_utc="$(date -u +%F)"
generated_utc="$(date -u +'%Y-%m-%dT%H:%M:%SZ')"

{
  echo "# Release Notes: ${TAG}"
  echo
  echo "- Release date: ${today_utc}"
  echo "- Gate: ${GATE}"
  echo "- Status: ${STATUS}"
  echo "- Scope: ${SCOPE}"
  echo
  echo "## Summary"
  echo
  echo "This package was generated automatically by \`scripts/generate_release_evidence_bundle.sh\`."
  echo
  echo "## Key Commits"
  echo
  idx=1
  for line in "${commit_lines[@]}"; do
    echo "${idx}. \`${line%% *}\` - ${line#* }"
    idx=$((idx + 1))
  done
} > "$release_notes_file"

{
  echo "# Evidence Bundle: ${TAG}"
  echo
  echo "- Generated: ${generated_utc}"
  echo "- Release tag: \`${TAG}\`"
  echo "- Gate: \`${GATE}\`"
  echo
  echo "## Verification Summary (Imported)"
  echo
  if [[ ${#verification_lines[@]} -eq 0 ]]; then
    echo "- No verification lines extracted from sprint report."
  else
    for line in "${verification_lines[@]}"; do
      echo "$line"
    done
  fi
  echo
  echo "## Artifacts"
  echo
  echo "1. Release notes:"
  echo "   - \`${release_notes_file}\`"
  echo "2. Bundle manifest:"
  echo "   - \`${manifest_file}\`"
  if [[ -n "$SPRINT_REPORT" && -f "$SPRINT_REPORT" ]]; then
    echo "3. Sprint report source:"
    echo "   - \`${SPRINT_REPORT}\`"
  fi
  if [[ -f "$PROGRESS_SNAPSHOT" ]]; then
    echo "4. Progress snapshot source:"
    echo "   - \`${PROGRESS_SNAPSHOT}\`"
  fi
} > "$evidence_bundle_file"

{
  echo "release:"
  echo "  tag: ${TAG}"
  echo "  date: \"${today_utc}\""
  echo "  gate: ${GATE}"
  echo "  status: ${STATUS}"
  echo "  generated_at: \"${generated_utc}\""
  echo
  echo "commits:"
  for id in "${commit_ids[@]}"; do
    echo "  - ${id}"
  done
  echo
  echo "evidence:"
  echo "  - ${release_notes_file}"
  echo "  - ${evidence_bundle_file}"
  if [[ -n "$SPRINT_REPORT" && -f "$SPRINT_REPORT" ]]; then
    echo "  - ${SPRINT_REPORT}"
  fi
  if [[ -f "$PROGRESS_SNAPSHOT" ]]; then
    echo "  - ${PROGRESS_SNAPSHOT}"
  fi
} > "$manifest_file"

echo "Generated release evidence bundle:"
echo "- ${release_notes_file}"
echo "- ${evidence_bundle_file}"
echo "- ${manifest_file}"

