import { test, expect, signIn } from './fixtures.js';

test('administrator manages persistent providers and environment rows remain read only', async ({ page, service }) => {
  await signIn(page, service);
  await page.locator('#nav-oauth').click();
  const environment = page.locator('tbody tr').filter({ hasText: 'github' });
  await expect(environment.locator('button')).toHaveCount(3);
  for (const button of await environment.locator('button').all()) await expect(button).toBeDisabled();
  await page.locator('#oauth-kind').selectOption('oauth2');
  for (const [field, value] of Object.entries({
    name: 'browser-local', client_id: 'browser-client', client_secret: 'browser-secret',
    authorization_url: `${service.url}/authorize`, token_url: `${service.url}/token`,
    userinfo_url: `${service.url}/userinfo`, scopes: 'read:user',
  })) await page.locator(`#oauth-${field}`).fill(value);
  const created = page.waitForResponse(response => response.url().endsWith('/api/admin/oauth/providers') && response.request().method() === 'POST');
  await page.locator('#save-oauth-provider').click();
  expect((await created).status()).toBe(201);
  const row = page.locator('tbody tr').filter({ hasText: 'browser-local' });
  await expect(row).toBeVisible();
  await row.locator('[data-edit-oauth]').click();
  await expect(page.locator('#oauth-name')).toHaveAttribute('readonly', '');
  await expect(page.locator('#oauth-client_secret')).toHaveValue('');
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
  await expect(page.locator('#oauth-scopes')).toHaveValue('read:user user:email');
  await page.locator('#cancel-oauth-provider').click();
  await row.locator('[data-delete-oauth]').click();
  await expect(row).toHaveCount(0);
});
