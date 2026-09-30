import { test, expect, signIn } from './fixtures.js';

test('same-origin Cookie login removes old Bearer caches and restores after refresh', async ({ page, service }) => {
  await page.goto(service.uiUrl);
  await page.evaluate(() => {
    localStorage.setItem('rustdesk_api_token', 'obsolete-bearer');
    sessionStorage.setItem('rustdesk_api_token', 'obsolete-bearer');
  });
  const requests = [];
  page.on('request', request => { if (request.url().includes('/api/')) requests.push(request); });
  await signIn(page, service);
  expect(await page.evaluate(() => [localStorage.getItem('rustdesk_api_token'), sessionStorage.getItem('rustdesk_api_token')])).toEqual([null, null]);
  await page.reload();
  await expect(page.locator('#nav-oauth')).toBeVisible();
  const missingCsrf = await page.evaluate(async () => (await fetch('/api/logout', { method: 'POST', credentials: 'include' })).status);
  expect(missingCsrf).toBe(403);
  await expect(page.locator('#nav-oauth')).toBeVisible();
  await page.route('**/api/logout', route => route.abort(), { times: 1 });
  await page.locator('#logout').click();
  await expect(page.locator('.notice')).toContainText(/无法连接 API|Cannot connect|API service/i);
  await expect(page.locator('#nav-oauth')).toBeVisible();
  const logout = page.waitForResponse(response => response.url().endsWith('/api/logout'));
  await page.locator('#logout').click();
  expect((await logout).status()).toBe(200);
  await expect(page.locator('#auth-form')).toBeVisible();
  expect(requests.every(request => !request.headers().authorization)).toBe(true);
  expect(requests.filter(request => request.url().endsWith('/api/logout')).at(-1).headers()['x-csrf-token']).toBeTruthy();
  await page.reload();
  await expect(page.locator('#auth-form')).toBeVisible();
});

test.describe('same-site cross-origin Web', () => {
  test.use({ topology: 'allowed' });
  test('preflight, login, refresh, provider write and logout use the permitted Origin', async ({ page, service }) => {
    await signIn(page, service);
    await page.reload();
    await expect(page.locator('#nav-oauth')).toBeVisible();
    await page.locator('#nav-oauth').click();
    await page.locator('#oauth-kind').selectOption('oauth2');
    for (const [field, value] of Object.entries({ name: 'cross-origin', client_id: 'client', client_secret: 'secret',
      authorization_url: `${service.url}/authorize`, token_url: `${service.url}/token`, userinfo_url: `${service.url}/userinfo` })) {
      await page.locator(`#oauth-${field}`).fill(value);
    }
    const create = page.waitForResponse(response => response.url().endsWith('/api/admin/oauth/providers') && response.request().method() === 'POST');
    await page.locator('#save-oauth-provider').click();
    const response = await create;
    expect(response.status()).toBe(201);
    expect(response.headers()['access-control-allow-origin']).toBe(service.uiUrl);
    expect(response.headers()['access-control-allow-credentials']).toBe('true');
    await expect(page.locator('tbody tr').filter({ hasText: 'cross-origin' })).toBeVisible();
    await page.locator('#logout').click();
    await expect(page.locator('#auth-form')).toBeVisible();
    // The real browser reaching a JSON+CSRF mutation confirms its preflight succeeded.
  });
});

test.describe('unapproved cross-origin Web', () => {
  test.use({ topology: 'denied' });
  test('browser cannot log in through an unapproved Origin', async ({ page, service }) => {
    await page.goto(service.uiUrl);
    await page.locator('#username').fill('browser-admin');
    await page.locator('#password').fill('temporary-browser-password');
    await page.locator('#auth-submit').click();
    await expect(page.locator('#nav-oauth')).toHaveCount(0);
    await expect(page.locator('.notice')).toContainText(/无法连接 API|Cannot connect|API service/i);
  });
});

test.describe('cross-site Cookie restrictions', () => {
  test.use({ topology: 'cross-site' });
  test('blocked third-party Cookies produce a clear session error', async ({ page, service, context }) => {
    const cdp = await context.newCDPSession(page);
    await cdp.send('Network.enable');
    await cdp.send('Network.setCookieControls', {
      enableThirdPartyCookieRestriction: true,
      disableThirdPartyCookieMetadata: true,
      disableThirdPartyCookieHeuristics: true,
    });
    await page.goto(service.uiUrl);
    await page.locator('#username').fill('browser-admin');
    await page.locator('#password').fill('temporary-browser-password');
    await page.locator('#auth-submit').click();
    await expect(page.locator('.notice')).toContainText(/Cookie 会话不可用|Cookie session unavailable/);
    await expect(page.locator('#nav-oauth')).toHaveCount(0);
  });
});
