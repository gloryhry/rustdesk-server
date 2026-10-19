import { test, expect, signIn } from './fixtures.js';

async function openTab(page, name) {
  await page.locator(`#oauth-tabs [data-tab="${name}"]`).click();
  await expect(page.locator(`[data-oauth-panel="${name}"]`)).toBeVisible();
}

test('administrator manages persistent providers and environment rows remain read only', async ({ page, service }) => {
  await signIn(page, service);
  await page.locator('#nav-oauth').click();
  const environment = page.locator('tbody tr').filter({ hasText: 'github' });
  await expect(environment.locator('button')).toHaveCount(3);
  for (const button of await environment.locator('button').all()) await expect(button).toBeDisabled();

  await page.locator('#oauth-kind').selectOption('oauth2');
  await page.locator('#oauth-name').fill('browser-local');
  await openTab(page, 'client');
  await page.locator('#oauth-client_id').fill('browser-client');
  await page.locator('#oauth-client_secret').fill('browser-secret');
  await openTab(page, 'endpoints');
  await page.locator('#oauth-authorization_url').fill(`${service.url}/authorize`);
  await page.locator('#oauth-token_url').fill(`${service.url}/token`);
  await page.locator('#oauth-userinfo_url').fill(`${service.url}/userinfo`);
  await openTab(page, 'scope');
  await page.locator('#oauth-scopes').fill('read:user');

  const created = page.waitForResponse(response => response.url().endsWith('/api/admin/oauth/providers') && response.request().method() === 'POST');
  await page.locator('#save-oauth-provider').click();
  expect((await created).status()).toBe(201);
  const row = page.locator('tbody tr').filter({ hasText: 'browser-local' });
  await expect(row).toBeVisible();

  await row.locator('[data-edit-oauth]').click();
  await expect(page.locator('#oauth-name')).toHaveAttribute('readonly', '');
  await openTab(page, 'client');
  await expect(page.locator('#oauth-client_secret')).toHaveValue('');
  await openTab(page, 'scope');
  await page.locator('#oauth-scopes').fill('read:user user:email');
  const updated = page.waitForResponse(response => response.url().endsWith('/api/admin/oauth/providers/update'));
  await page.locator('#save-oauth-provider').click();
  expect((await updated).status()).toBe(200);

  await row.locator('[data-toggle-oauth]').click();
  await expect(row.locator('[data-toggle-oauth]')).toHaveAttribute('data-oauth-enabled', 'false');
  await row.locator('[data-toggle-oauth]').click();
  await expect(row.locator('[data-toggle-oauth]')).toHaveAttribute('data-oauth-enabled', 'true');
  await page.reload();
  await page.locator('#nav-oauth').click();
  await row.locator('[data-edit-oauth]').click();
  await openTab(page, 'scope');
  await expect(page.locator('#oauth-scopes')).toHaveValue('read:user user:email');
  await page.locator('#cancel-oauth-provider').click();
  await row.locator('[data-delete-oauth]').click();
  await expect(row).toHaveCount(0);
});

test('a missing required OAuth field switches to its tab before submission', async ({ page, service }) => {
  await signIn(page, service);
  await page.locator('#nav-oauth').click();
  await openTab(page, 'scope');
  await page.locator('#save-oauth-provider').click();
  await expect(page.locator('#oauth-tabs [data-tab="identity"]')).toHaveClass(/active/);
  await expect(page.locator('#oauth-name')).toBeFocused();
  await expect(page.locator('#oauth-name')).toHaveAttribute('aria-invalid', 'true');
  await expect(page.locator('.notice')).toContainText('必填项');
});
