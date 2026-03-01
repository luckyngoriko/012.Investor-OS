#!/usr/bin/env bash
set -euo pipefail

PROJECT_NAME="${PROJECT_NAME:-unknown}"
STABLE_WARNING_LOG="${STABLE_WARNING_LOG:-frontend/investor-dashboard/warning-budget-stable-${PROJECT_NAME}.jsonl}"
QUARANTINE_WARNING_LOG="${QUARANTINE_WARNING_LOG:-frontend/investor-dashboard/warning-budget-quarantine-${PROJECT_NAME}.jsonl}"
CHART_WARNING_BUDGET="${CHART_WARNING_BUDGET:-0}"
OTHER_WARNING_BUDGET="${OTHER_WARNING_BUDGET:-0}"
IGNORED_OTHER_WARNING_SUBSTRING="${IGNORED_OTHER_WARNING_SUBSTRING:-Layout was forced before the page was fully loaded.}"
OUTPUT_MD="${OUTPUT_MD:-frontend/investor-dashboard/warning-budget-${PROJECT_NAME}.md}"
OUTPUT_JSON="${OUTPUT_JSON:-frontend/investor-dashboard/warning-budget-${PROJECT_NAME}.json}"

load_entries_from_log() {
  local log_file="$1"
  if [[ -f "$log_file" ]] && [[ -s "$log_file" ]]; then
    jq -s '.' "$log_file"
  else
    echo "[]"
  fi
}

stable_entries_json="$(load_entries_from_log "$STABLE_WARNING_LOG")"
quarantine_entries_json="$(load_entries_from_log "$QUARANTINE_WARNING_LOG")"
combined_entries_json="$(jq -n --argjson stable "$stable_entries_json" --argjson quarantine "$quarantine_entries_json" '$stable + $quarantine')"

ignored_other_warnings="$(jq --arg ignored_substring "$IGNORED_OTHER_WARNING_SUBSTRING" '
  [
    .[].warnings[]?
    | select(
        .category == "other"
        and ((.text // "") | contains($ignored_substring))
      )
  ]
  | length
' <<<"$combined_entries_json")"

filtered_entries_json="$(jq --arg ignored_substring "$IGNORED_OTHER_WARNING_SUBSTRING" '
  map(
    .warnings = [
      (.warnings // [])[]
      | select(
          (
            .category == "other"
            and ((.text // "") | contains($ignored_substring))
          ) | not
        )
    ]
    | .warning_count = (.warnings | length)
    | .category_counts = {
        chart_container: ([.warnings[]? | select(.category == "chart_container")] | length),
        other: ([.warnings[]? | select(.category == "other")] | length)
      }
  )
' <<<"$combined_entries_json")"

total_tests="$(jq '[.[].test_title] | length' <<<"$filtered_entries_json")"
total_warnings="$(jq '[.[].warning_count] | add // 0' <<<"$filtered_entries_json")"
chart_warnings="$(jq '[.[].category_counts.chart_container // 0] | add // 0' <<<"$filtered_entries_json")"
other_warnings="$(jq '[.[].category_counts.other // 0] | add // 0' <<<"$filtered_entries_json")"

verdict="pass"
if (( chart_warnings > CHART_WARNING_BUDGET )); then
  verdict="fail"
fi
if (( other_warnings > OTHER_WARNING_BUDGET )); then
  verdict="fail"
fi

top_warning_tests_json="$(jq '
  [
    .[]
    | select((.warning_count // 0) > 0)
    | {
        test_title: .test_title,
        warning_count: (.warning_count // 0),
        chart_container: (.category_counts.chart_container // 0),
        other: (.category_counts.other // 0)
      }
  ]
  | sort_by(-.warning_count)
  | .[:20]
' <<<"$filtered_entries_json")"

top_chart_messages_json="$(jq '
  [
    .[].warnings[]?
    | select(.category == "chart_container")
    | .text
  ]
  | group_by(.)
  | map({ message: .[0], count: length })
  | sort_by(-.count)
  | .[:5]
' <<<"$filtered_entries_json")"

top_other_messages_json="$(jq '
  [
    .[].warnings[]?
    | select(.category == "other")
    | .text
  ]
  | group_by(.)
  | map({ message: .[0], count: length })
  | sort_by(-.count)
  | .[:10]
' <<<"$filtered_entries_json")"

mkdir -p "$(dirname "$OUTPUT_MD")"
mkdir -p "$(dirname "$OUTPUT_JSON")"

{
  echo "# Warning Budget Report (${PROJECT_NAME})"
  echo
  echo "- Generated: $(date -u +'%Y-%m-%dT%H:%M:%SZ')"
  echo "- Verdict: \`${verdict}\`"
  echo "- Chart warning budget: \`${CHART_WARNING_BUDGET}\`"
  echo "- Other warning budget: \`${OTHER_WARNING_BUDGET}\`"
  echo "- Chart warnings observed: \`${chart_warnings}\`"
  echo "- Other warnings observed: \`${other_warnings}\`"
  echo "- Ignored known other warnings: \`${ignored_other_warnings}\`"
  echo "- Total warnings observed: \`${total_warnings}\`"
  echo "- Tests inspected: \`${total_tests}\`"
  echo
  echo "## Warning-Heavy Tests"
  echo
  if [[ "$(jq 'length' <<<"$top_warning_tests_json")" -eq 0 ]]; then
    echo "- No warnings captured."
  else
    echo "| warning_count | chart_container | other | test |"
    echo "|---:|---:|---:|---|"
    jq -r '.[] | "| \(.warning_count) | \(.chart_container) | \(.other) | \(.test_title) |"' <<<"$top_warning_tests_json"
  fi
} > "$OUTPUT_MD"

jq -n \
  --arg project "$PROJECT_NAME" \
  --arg verdict "$verdict" \
  --argjson chart_budget "$CHART_WARNING_BUDGET" \
  --argjson other_budget "$OTHER_WARNING_BUDGET" \
  --argjson chart_warnings "$chart_warnings" \
  --argjson other_warnings "$other_warnings" \
  --argjson ignored_other_warnings "$ignored_other_warnings" \
  --argjson total_warnings "$total_warnings" \
  --argjson total_tests "$total_tests" \
  --argjson stable_entries "$stable_entries_json" \
  --argjson quarantine_entries "$quarantine_entries_json" \
  --argjson top_warning_tests "$top_warning_tests_json" \
  --arg ignored_other_warning_substring "$IGNORED_OTHER_WARNING_SUBSTRING" \
  '{
      project: $project,
      verdict: $verdict,
      chart_warning_budget: $chart_budget,
      other_warning_budget: $other_budget,
      ignored_other_warning_substring: $ignored_other_warning_substring,
      stats: {
        total_tests: $total_tests,
        total_warnings: $total_warnings,
        chart_container_warnings: $chart_warnings,
        other_warnings: $other_warnings,
        ignored_known_other_warnings: $ignored_other_warnings
      },
      top_warning_tests: $top_warning_tests,
      entries: {
        stable: $stable_entries,
        quarantine: $quarantine_entries
      }
    }' > "$OUTPUT_JSON"

echo "Warning budget report written:"
echo "- $OUTPUT_MD"
echo "- $OUTPUT_JSON"
echo "Warning summary: tests=${total_tests}, total=${total_warnings}, chart=${chart_warnings}/${CHART_WARNING_BUDGET}, other=${other_warnings}/${OTHER_WARNING_BUDGET}, ignored_other=${ignored_other_warnings}"
if [[ "$(jq 'length' <<<"$top_chart_messages_json")" -gt 0 ]]; then
  echo "Top chart warning messages:"
  jq -r '.[] | "- (\(.count)x) \(.message)"' <<<"$top_chart_messages_json"
fi
if [[ "$(jq 'length' <<<"$top_other_messages_json")" -gt 0 ]]; then
  echo "Top other warning messages:"
  jq -r '.[] | "- (\(.count)x) \(.message)"' <<<"$top_other_messages_json"
fi

if [[ "$verdict" != "pass" ]]; then
  if (( chart_warnings > CHART_WARNING_BUDGET )); then
    echo "Chart warning budget exceeded: observed=${chart_warnings}, budget=${CHART_WARNING_BUDGET}"
  fi
  if (( other_warnings > OTHER_WARNING_BUDGET )); then
    echo "Other warning budget exceeded: observed=${other_warnings}, budget=${OTHER_WARNING_BUDGET}"
  fi
  exit 1
fi
