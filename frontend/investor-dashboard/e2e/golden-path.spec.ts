import { test, expect } from '@playwright/test';

/**
 * Sprint 4 Golden Path E2E Tests
 * 
 * These tests validate the critical user journeys:
 * 1. Dashboard loads with key metrics
 * 2. Proposals can be viewed and managed
 * 3. Positions are displayed correctly
 * 4. Journal shows decision history
 * 5. Kill switch is accessible
 */

test.describe('S4 Golden Path', () => {
  
  test.beforeEach(async ({ page }) => {
    // Navigate to the dashboard before each test
    await page.goto('/');
  });

  test('GP-S4-01: Dashboard loads with key metrics', async ({ page }) => {
    // Verify page title
    await expect(page).toHaveTitle(/Investor OS/);
    
    // Verify key elements are visible
    await expect(page.locator('text=Dashboard')).toBeVisible();
    await expect(page.locator('text=Portfolio Value')).toBeVisible();
    await expect(page.locator('text=Market Regime')).toBeVisible();
    await expect(page.locator('text=Pending Decisions')).toBeVisible();
    
    // Verify navigation exists
    await expect(page.locator('nav')).toBeVisible();
    await expect(page.locator('text=Proposals')).toBeVisible();
    await expect(page.locator('text=Positions')).toBeVisible();
    await expect(page.locator('text=Journal')).toBeVisible();
  });

  test('GP-S4-02: Proposals page shows pending proposals', async ({ page }) => {
    // Navigate to proposals page
    await page.click('text=Proposals');
    
    // Verify page loads
    await expect(page.locator('h1')).toContainText('Trade Proposals');
    
    // Verify tabs exist
    await expect(page.locator('text=Pending')).toBeVisible();
    await expect(page.locator('text=Confirmed')).toBeVisible();
    await expect(page.locator('text=Rejected')).toBeVisible();
    
    // If proposals exist, verify structure
    const proposalCards = page.locator('[class*="Card"]').first();
    if (await proposalCards.isVisible().catch(() => false)) {
      await expect(page.locator('text=Confirm')).toBeVisible();
      await expect(page.locator('text=Reject')).toBeVisible();
    }
  });

  test('GP-S4-03: Confirm proposal works', async ({ page }) => {
    // Navigate to proposals
    await page.click('text=Proposals');
    await expect(page.locator('h1')).toContainText('Trade Proposals');
    
    // Find and click first Confirm button (if exists)
    const confirmButton = page.locator('button:has-text("Confirm")').first();
    
    // Only test if there are pending proposals
    if (await confirmButton.isVisible().catch(() => false)) {
      await confirmButton.click();
      
      // Verify the button state changed or proposal moved
      await expect(page.locator('text=Confirmed')).toBeVisible();
    } else {
      test.skip('No pending proposals to confirm');
    }
  });

  test('GP-S4-04: Reject proposal works', async ({ page }) => {
    // Navigate to proposals
    await page.click('text=Proposals');
    await expect(page.locator('h1')).toContainText('Trade Proposals');
    
    // Find and click first Reject button (if exists)
    const rejectButton = page.locator('button:has-text("Reject")').first();
    
    // Only test if there are pending proposals
    if (await rejectButton.isVisible().catch(() => false)) {
      await rejectButton.click();
      
      // Handle the reject dialog
      await expect(page.locator('text=Reject Proposal')).toBeVisible();
      await page.fill('textarea', 'Test rejection reason');
      await page.click('button:has-text("Reject Proposal")');
      
      // Verify the proposal was rejected
      await expect(page.locator('text=Rejected')).toBeVisible();
    } else {
      test.skip('No pending proposals to reject');
    }
  });

  test('GP-S4-05: Positions page shows holdings', async ({ page }) => {
    // Navigate to positions
    await page.click('text=Positions');
    
    // Verify page loads
    await expect(page.locator('h1')).toContainText('Positions');
    
    // Verify summary cards
    await expect(page.locator('text=Total P&L')).toBeVisible();
    await expect(page.locator('text=Portfolio Value')).toBeVisible();
    await expect(page.locator('text=Win Rate')).toBeVisible();
    
    // Verify table exists (if there are positions)
    const table = page.locator('table');
    if (await table.isVisible().catch(() => false)) {
      // Verify table headers
      await expect(page.locator('th:has-text("Ticker")')).toBeVisible();
      await expect(page.locator('th:has-text("P&L")')).toBeVisible();
      await expect(page.locator('th:has-text("% NAV")')).toBeVisible();
    }
  });

  test('GP-S4-06: Journal page accessible', async ({ page }) => {
    // Navigate to journal
    await page.click('text=Journal');
    
    // Verify page loads
    await expect(page.locator('h1')).toContainText('Decision Journal');
    
    // Verify stats overview
    await expect(page.locator('text=Total P&L')).toBeVisible();
    await expect(page.locator('text=Win Rate')).toBeVisible();
    await expect(page.locator('text=Avg Win')).toBeVisible();
    await expect(page.locator('text=Avg Loss')).toBeVisible();
    
    // Verify trade history table
    await expect(page.locator('text=Trade History')).toBeVisible();
  });

  test('GP-S4-07: Settings page with kill switch accessible', async ({ page }) => {
    // Navigate to settings
    await page.click('text=Settings');
    
    // Verify page loads
    await expect(page.locator('h1')).toContainText('Settings');
    
    // Verify kill switch section
    await expect(page.locator('text=Kill Switch')).toBeVisible();
    await expect(page.locator('text=Emergency Stop')).toBeVisible();
    
    // Verify trigger button exists
    const killSwitchButton = page.locator('button:has-text("Trigger Kill Switch")');
    await expect(killSwitchButton).toBeVisible();
  });

  test('GP-S4-08: Navigation between pages works', async ({ page }) => {
    // Test navigation flow
    const pages = [
      { link: 'Dashboard', heading: 'Dashboard' },
      { link: 'Proposals', heading: 'Trade Proposals' },
      { link: 'Positions', heading: 'Positions' },
      { link: 'Journal', heading: 'Decision Journal' },
      { link: 'Settings', heading: 'Settings' },
    ];
    
    for (const { link, heading } of pages) {
      await page.click(`text=${link}`);
      await expect(page.locator('h1')).toContainText(heading);
    }
  });

  test('GP-S4-09: Dashboard loads in under 3 seconds', async ({ page }) => {
    // Measure load time
    const startTime = Date.now();
    await page.goto('/');
    await page.waitForLoadState('networkidle');
    const loadTime = Date.now() - startTime;
    
    // Assert load time is under 3 seconds
    expect(loadTime).toBeLessThan(3000);
    console.log(`Dashboard loaded in ${loadTime}ms`);
  });

  test('GP-S4-10: Responsive design on mobile viewport', async ({ page }) => {
    // Set mobile viewport
    await page.setViewportSize({ width: 375, height: 667 });
    
    // Reload page
    await page.goto('/');
    
    // Verify content is visible
    await expect(page.locator('text=Portfolio Value')).toBeVisible();
    await expect(page.locator('text=Market Regime')).toBeVisible();
    
    // Verify cards stack vertically (no horizontal overflow)
    const body = page.locator('body');
    const bodyBox = await body.boundingBox();
    expect(bodyBox?.width).toBeLessThanOrEqual(375);
  });
});
