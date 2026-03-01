import { test as base, expect } from "@playwright/test";
import { appendFileSync, mkdirSync } from "node:fs";
import { dirname } from "node:path";

type WarningCategory = "chart_container" | "other";

interface CapturedWarning {
  category: WarningCategory;
  text: string;
  location?: string;
}

interface WarningBudgetEntry {
  project: string;
  test_file: string;
  test_title: string;
  warning_count: number;
  category_counts: Record<WarningCategory, number>;
  warnings: CapturedWarning[];
}

const CHART_CONTAINER_WARNING_PATTERNS: RegExp[] = [
  /the width\([^)]*\) and height\([^)]*\) of chart should be greater than 0/i,
];

function classifyWarning(text: string): WarningCategory {
  if (CHART_CONTAINER_WARNING_PATTERNS.some((pattern) => pattern.test(text))) {
    return "chart_container";
  }
  return "other";
}

function appendWarningEntry(path: string, entry: WarningBudgetEntry) {
  mkdirSync(dirname(path), { recursive: true });
  appendFileSync(path, `${JSON.stringify(entry)}\n`, "utf8");
}

export const test = base.extend({
  page: async ({ page }, use, testInfo) => {
    const warnings: CapturedWarning[] = [];

    page.on("console", (message) => {
      if (message.type() !== "warning") {
        return;
      }

      const text = message.text().trim();
      warnings.push({
        category: classifyWarning(text),
        text,
        location: message.location()?.url,
      });
    });

    await use(page);

    const categoryCounts: Record<WarningCategory, number> = {
      chart_container: 0,
      other: 0,
    };
    warnings.forEach((warning) => {
      categoryCounts[warning.category] += 1;
    });

    const logPath =
      process.env.PW_WARNING_BUDGET_LOG ||
      `warning-budget-${testInfo.project.name}.jsonl`;

    appendWarningEntry(logPath, {
      project: testInfo.project.name,
      test_file: testInfo.file,
      test_title: testInfo.titlePath.join(" > "),
      warning_count: warnings.length,
      category_counts: categoryCounts,
      warnings,
    });
  },
});

export { expect };
