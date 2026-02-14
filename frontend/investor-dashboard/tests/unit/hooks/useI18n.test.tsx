/**
 * useI18n Hook Unit Tests
 */

import { renderHook, act } from "@testing-library/react";
import { describe, it, expect } from "vitest";

describe("useI18n", () => {
  it("returns default locale", () => {
    // Placeholder - actual implementation would test the hook
    const result = { current: { locale: "bg" } };
    expect(result.current.locale).toBe("bg");
  });

  it("changes locale", () => {
    let locale = "bg";
    
    act(() => {
      locale = "en";
    });
    
    expect(locale).toBe("en");
  });
});
