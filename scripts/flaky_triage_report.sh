#!/usr/bin/env bash
set -euo pipefail

PROJECT_NAME="${PROJECT_NAME:-unknown}"
STABLE_JSON="${STABLE_JSON:-frontend/investor-dashboard/test-results-stable-${PROJECT_NAME}.json}"
QUARANTINE_JSON="${QUARANTINE_JSON:-frontend/investor-dashboard/test-results-quarantine-${PROJECT_NAME}.json}"
QUARANTINE_MANIFEST="${QUARANTINE_MANIFEST:-frontend/investor-dashboard/tests/e2e/flaky-quarantine.json}"
STABLE_EXIT_CODE="${STABLE_EXIT_CODE:-0}"
QUARANTINE_EXIT_CODE="${QUARANTINE_EXIT_CODE:-0}"
OUTPUT_MD="${OUTPUT_MD:-frontend/investor-dashboard/flaky-triage-${PROJECT_NAME}.md}"
OUTPUT_JSON="${OUTPUT_JSON:-frontend/investor-dashboard/flaky-triage-${PROJECT_NAME}.json}"

count_stat() {
  local file="$1"
  local key="$2"
  if [[ -f "$file" ]]; then
    jq -r ".stats.${key} // 0" "$file"
  else
    echo "0"
  fi
}

collect_unstable_tests() {
  local file="$1"
  if [[ -f "$file" ]]; then
    jq -c '
      [
        .. | objects
        | select(has("tests"))
        | .tests[]?
        | select((.status // "") == "unexpected" or (.status // "") == "flaky")
        | {
            project: (.projectName // "unknown"),
            file: (.location.file // "unknown"),
            line: (.location.line // 0),
            title: (.title // "unknown"),
            status: (.status // "unknown")
          }
      ]
    ' "$file"
  else
    echo "[]"
  fi
}

stable_expected="$(count_stat "$STABLE_JSON" "expected")"
stable_unexpected="$(count_stat "$STABLE_JSON" "unexpected")"
stable_flaky="$(count_stat "$STABLE_JSON" "flaky")"
stable_skipped="$(count_stat "$STABLE_JSON" "skipped")"

quarantine_expected="$(count_stat "$QUARANTINE_JSON" "expected")"
quarantine_unexpected="$(count_stat "$QUARANTINE_JSON" "unexpected")"
quarantine_flaky="$(count_stat "$QUARANTINE_JSON" "flaky")"
quarantine_skipped="$(count_stat "$QUARANTINE_JSON" "skipped")"

stable_unstable_json="$(collect_unstable_tests "$STABLE_JSON")"
quarantine_unstable_json="$(collect_unstable_tests "$QUARANTINE_JSON")"

if [[ -f "$QUARANTINE_MANIFEST" ]]; then
  quarantine_entries_json="$(jq -c '.entries // []' "$QUARANTINE_MANIFEST")"
else
  quarantine_entries_json="[]"
fi

verdict="clean"
if [[ "$STABLE_EXIT_CODE" != "0" ]]; then
  verdict="blocking-stable-failures"
elif [[ "$QUARANTINE_EXIT_CODE" != "0" ]]; then
  verdict="quarantine-failures-non-blocking"
fi

mkdir -p "$(dirname "$OUTPUT_MD")"
mkdir -p "$(dirname "$OUTPUT_JSON")"

{
  echo "# Flaky Triage Report (${PROJECT_NAME})"
  echo
  echo "- Generated: $(date -u +'%Y-%m-%dT%H:%M:%SZ')"
  echo "- Verdict: \`${verdict}\`"
  echo "- Stable exit code: \`${STABLE_EXIT_CODE}\`"
  echo "- Quarantine exit code: \`${QUARANTINE_EXIT_CODE}\`"
  echo
  echo "## Stable Suite Stats"
  echo
  echo "| expected | unexpected | flaky | skipped |"
  echo "|---:|---:|---:|---:|"
  echo "| ${stable_expected} | ${stable_unexpected} | ${stable_flaky} | ${stable_skipped} |"
  echo
  echo "## Quarantined Suite Stats"
  echo
  echo "| expected | unexpected | flaky | skipped |"
  echo "|---:|---:|---:|---:|"
  echo "| ${quarantine_expected} | ${quarantine_unexpected} | ${quarantine_flaky} | ${quarantine_skipped} |"
  echo
  echo "## Quarantine Entries"
  echo
  if [[ "$(jq 'length' <<<"$quarantine_entries_json")" -eq 0 ]]; then
    echo "- No quarantined specs configured."
  else
    echo "| spec | owner | reason | expires | ticket |"
    echo "|---|---|---|---|---|"
    jq -r '.[] | "| `\(.spec // "n/a")` | \(.owner // "n/a") | \(.reason // "n/a") | \(.expires // "n/a") | \(.ticket // "n/a") |"' <<<"$quarantine_entries_json"
  fi
  echo
  echo "## Unstable Tests (Stable Suite)"
  echo
  if [[ "$(jq 'length' <<<"$stable_unstable_json")" -eq 0 ]]; then
    echo "- None."
  else
    echo "| status | project | location | title |"
    echo "|---|---|---|---|"
    jq -r '.[] | "| `\(.status)` | `\(.project)` | `\(.file):\(.line)` | \(.title) |"' <<<"$stable_unstable_json"
  fi
  echo
  echo "## Unstable Tests (Quarantined Suite)"
  echo
  if [[ "$(jq 'length' <<<"$quarantine_unstable_json")" -eq 0 ]]; then
    echo "- None."
  else
    echo "| status | project | location | title |"
    echo "|---|---|---|---|"
    jq -r '.[] | "| `\(.status)` | `\(.project)` | `\(.file):\(.line)` | \(.title) |"' <<<"$quarantine_unstable_json"
  fi
} > "$OUTPUT_MD"

jq -n \
  --arg project "$PROJECT_NAME" \
  --arg verdict "$verdict" \
  --argjson stable_exit "$STABLE_EXIT_CODE" \
  --argjson quarantine_exit "$QUARANTINE_EXIT_CODE" \
  --argjson stable_expected "$stable_expected" \
  --argjson stable_unexpected "$stable_unexpected" \
  --argjson stable_flaky "$stable_flaky" \
  --argjson stable_skipped "$stable_skipped" \
  --argjson quarantine_expected "$quarantine_expected" \
  --argjson quarantine_unexpected "$quarantine_unexpected" \
  --argjson quarantine_flaky "$quarantine_flaky" \
  --argjson quarantine_skipped "$quarantine_skipped" \
  --argjson quarantine_entries "$quarantine_entries_json" \
  --argjson stable_unstable "$stable_unstable_json" \
  --argjson quarantine_unstable "$quarantine_unstable_json" \
  '{
      project: $project,
      verdict: $verdict,
      stable_exit_code: $stable_exit,
      quarantine_exit_code: $quarantine_exit,
      stats: {
        stable: {
          expected: $stable_expected,
          unexpected: $stable_unexpected,
          flaky: $stable_flaky,
          skipped: $stable_skipped
        },
        quarantine: {
          expected: $quarantine_expected,
          unexpected: $quarantine_unexpected,
          flaky: $quarantine_flaky,
          skipped: $quarantine_skipped
        }
      },
      quarantine_entries: $quarantine_entries,
      unstable_tests: {
        stable: $stable_unstable,
        quarantine: $quarantine_unstable
      }
    }' > "$OUTPUT_JSON"

echo "Flaky triage report written:"
echo "- $OUTPUT_MD"
echo "- $OUTPUT_JSON"

