const { test, expect } = require('@playwright/test');

test('accounts, partials, safe storage, refresh, mobile layout and logout', async ({ page }) => {
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  page.on('console', message => {
    if (message.type() === 'error' && /content security policy/i.test(message.text())) errors.push(message.text());
  });
  await page.goto('/');
  await expect(page.locator('h1')).toHaveText('web-template');
  await expect(page.getByRole('link', { name: 'Sign in', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Load partial content', exact: true }).click();
  await expect(page.locator('#partial-content')).toContainText('Counter: 1');
  await page.getByRole('button', { name: 'Load partial content', exact: true }).click();
  await expect(page.locator('#partial-content')).toContainText('Counter: 2');

  await page.goto('/register');
  await page.getByLabel('Username', { exact: true }).fill('browser_user');
  await page.getByLabel('Email', { exact: true }).fill('browser@example.test');
  await page.getByLabel('Password', { exact: true }).fill('long-password-123');
  await page.getByLabel('Confirm Password', { exact: true }).fill('long-password-123');
  await page.getByRole('button', { name: 'Create Account' }).click();
  await expect(page).toHaveURL('/');
  await expect(page.getByText('browser_user', { exact: true })).toBeVisible();

  const payload = '<img src=x onerror="window.injected=true">';
  await page.getByLabel('Key', { exact: true }).fill('markup');
  await page.getByLabel('Value', { exact: true }).fill(payload);
  // Pressing Enter must submit the visible named fields, not stale hidden copies.
  await page.getByLabel('Value', { exact: true }).press('Enter');
  await expect(page.locator('#storage-result')).toContainText(payload);
  await expect(page.locator('#storage-result img')).toHaveCount(0);
  await page.getByRole('link', { name: 'Load Data', exact: true }).click();
  await expect(page.locator('#storage-result')).toContainText('markup');
  expect(await page.evaluate(() => window.injected)).toBeUndefined();

  await page.getByRole('link', { name: 'Refresh Page', exact: true }).click();
  await expect(page.getByText('browser_user', { exact: true })).toBeVisible();
  await page.setViewportSize({ width: 375, height: 812 });
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await page.getByRole('button', { name: 'Logout', exact: true }).click();
  await expect(page).toHaveURL('/login');
  await page.goto('/profile');
  await expect(page).toHaveURL('/login');
  expect(errors).toEqual([]);
});

test('public navigation and registration work without JavaScript', async ({ browser }) => {
  const context = await browser.newContext({ javaScriptEnabled: false });
  const page = await context.newPage();
  await page.goto('http://localhost:3107/');
  await page.getByRole('link', { name: 'Register', exact: true }).click();
  await page.getByLabel('Username', { exact: true }).fill('nojs_user');
  await page.getByLabel('Email', { exact: true }).fill('nojs@example.test');
  await page.getByLabel('Password', { exact: true }).fill('long-password-123');
  await page.getByLabel('Confirm Password', { exact: true }).fill('long-password-123');
  await page.getByRole('button', { name: 'Create Account' }).click();
  await expect(page.getByText('nojs_user', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Logout', exact: true }).click();
  await expect(page).toHaveURL('http://localhost:3107/login');
  await context.close();
});
