# Test Implementation Guide

## 🚀 Quick Start

```bash
# Install test dependencies
npm install

# Run all tests
npm run test:all

# Run unit tests
npm run test

# Run unit tests with coverage
npm run test:coverage

# Run E2E tests
npm run test:e2e

# Run E2E tests with UI
npm run test:e2e:ui
```

---

## 📁 Test Structure

```
tests/
├── unit/                   # Unit tests (Vitest)
│   ├── components/         # React component tests
│   ├── hooks/             # Custom hook tests
│   └── utils/             # Utility function tests
├── integration/            # API integration tests
├── e2e/                   # End-to-end tests (Playwright)
│   ├── auth/              # Authentication flows
│   ├── dashboard/         # Dashboard tests
│   ├── trading/           # Trading flow tests
│   ├── navigation/        # Navigation tests
│   ├── performance/       # Performance tests
│   └── accessibility/     # a11y tests
├── fixtures/              # Test data factories
│   └── factories.ts       # Fake data generators
└── mocks/                 # Mock services
    └── handlers.ts        # MSW handlers
```

---

## 🧪 Test Types

### 1. Unit Tests (Vitest)

```bash
# Run all unit tests
npm run test

# Run in watch mode
npm run test:watch

# Run with coverage
npm run test:coverage
```

### 2. E2E Tests (Playwright)

```bash
# Run all E2E tests
npm run test:e2e

# Run with UI
npm run test:e2e:ui

# Run specific test
npm run test:e2e -- tests/e2e/auth/login.spec.ts

# Run on specific browser
npm run test:e2e -- --project=chromium
```

### 3. Smoke Tests

Quick validation of critical paths:

```bash
npm run test:e2e -- tests/e2e/auth/login.spec.ts
npm run test:e2e -- tests/e2e/dashboard/dashboard.spec.ts
```

---

## 🎭 Mock Data

### Using Factories

```typescript
import { factories } from "../fixtures/factories";

// Create a user
const user = factories.user.create();

// Create admin user
const admin = factories.user.admin();

// Create many positions
const positions = factories.position.createMany(10);

// Create profitable position
const profitable = factories.position.profitable();

// Create high confidence proposal
const proposal = factories.proposal.withHighConfidence();
```

### Mock API Responses

```typescript
import { http, HttpResponse } from "msw";

// In your test
http.get("/api/portfolio", () => {
  return HttpResponse.json(factories.portfolio.create());
});
```

---

## 📊 Coverage Requirements

| Category | Minimum |
|----------|---------|
| Lines | 80% |
| Functions | 80% |
| Branches | 80% |
| Statements | 80% |

---

## 🔄 CI/CD Integration

```yaml
# .github/workflows/test.yml
name: Test

on: [push, pull_request]

jobs:
  unit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-node@v3
      - run: npm ci
      - run: npm run test:coverage

  e2e:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-node@v3
      - run: npm ci
      - run: npx playwright install
      - run: npm run test:e2e
```

---

## 🐛 Debugging Tests

### Unit Tests

```bash
# Run with debug output
npm run test -- --reporter=verbose

# Run single file
npm run test -- Button.test.tsx

# Run with UI
npm run test:watch
```

### E2E Tests

```bash
# Run in headed mode
npm run test:e2e -- --headed

# Run with UI
npm run test:e2e:ui

# Run with debug
npm run test:e2e:debug

# Trace execution
npm run test:e2e -- --trace=on
```

---

## 📝 Writing Tests

### Unit Test Example

```typescript
import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import MyComponent from "@/components/my-component";

describe("MyComponent", () => {
  it("renders correctly", () => {
    render(<MyComponent />);
    expect(screen.getByText("Hello")).toBeInTheDocument();
  });

  it("handles click", () => {
    const handleClick = vi.fn();
    render(<MyComponent onClick={handleClick} />);
    fireEvent.click(screen.getByRole("button"));
    expect(handleClick).toHaveBeenCalled();
  });
});
```

### E2E Test Example

```typescript
import { test, expect } from "@playwright/test";

test("user can login", async ({ page }) => {
  await page.goto("/login");
  await page.getByLabel(/email/i).fill("test@test.com");
  await page.getByLabel(/password/i).fill("password");
  await page.getByRole("button", { name: /sign in/i }).click();
  await expect(page).toHaveURL("/");
});
```

---

## 🎯 Test Priorities

1. **P0 - Critical**: Auth, Trading, Core navigation
2. **P1 - High**: Dashboard, Settings, Notifications
3. **P2 - Medium**: Help system, Theme switching
4. **P3 - Low**: Edge cases, Error states

---

## 🔧 Troubleshooting

### Common Issues

**Tests failing in CI but passing locally**
- Check for race conditions
- Add proper waits
- Use `await expect().toBeVisible()` instead of `expect().toBeVisible()`

**Flaky tests**
- Add retry logic
- Use data-testid attributes
- Stabilize async operations

**Coverage not collecting**
- Check vitest.config.ts exclude patterns
- Ensure source maps are enabled
- Verify file paths match include patterns
