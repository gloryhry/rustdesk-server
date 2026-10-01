import { test, expect, signIn } from './fixtures.js';

for (const delayed of ['/api/session/csrf', '/api/login-options']) {
  test(`public OAuth login survives unauthenticated startup when ${delayed} is delayed`, async ({ page, service }) => {
    await page.route(`**${delayed}`, async route => {
      await new Promise(resolve => setTimeout(resolve, 400));
      await route.continue();
    });
    await page.goto(service.uiUrl);
    await page.waitForLoadState('networkidle');
    await expect(page.locator('[data-oauth-provider="github"]')).toBeVisible();
    await expect(page.locator('#auth-form')).toBeVisible();
  });
}

test('logout keeps public choices, refreshes provider changes and tolerates a temporary lookup failure', async ({ page, service, request }) => {
  await signIn(page, service);
  const login = await request.post(`${service.url}/api/admin/login`, {
    data: { username: 'browser-admin', password: 'temporary-browser-password', id: 'test-admin', uuid: 'test-admin' },
  });
  const token = (await login.json()).access_token;
  const headers = { Authorization: `Bearer ${token}` };
  const created = await page.request.post(`${service.url}/api/admin/oauth/providers`, {
    headers, data: { name: 'public-choice', kind: 'oauth2', enabled: true, client_id: 'test', client_secret: 'test-only',
      authorization_url: `${service.url}/authorize`, token_url: `${service.url}/token`,
      userinfo_url: `${service.url}/userinfo`, scopes: 'profile' },
  });
  expect(created.status()).toBe(201);
  const provider = await created.json();
  await page.locator('#logout').click();
  await expect(page.locator('[data-oauth-provider="github"]')).toBeVisible();
  await expect(page.locator('[data-oauth-provider="public-choice"]')).toBeVisible();
  await signIn(page, service);
  const disabled = await page.request.post(`${service.url}/api/admin/oauth/providers/toggle`, {
    headers, data: { id: provider.id, enabled: false },
  });
  expect(disabled.status()).toBe(200);
  await page.locator('#logout').click();
  await expect(page.locator('#auth-form')).toBeVisible();
  await expect(page.locator('[data-oauth-provider="github"]')).toBeVisible();
  await expect(page.locator('[data-oauth-provider="public-choice"]')).toHaveCount(0);
  await signIn(page, service);
  await page.route('**/api/login-options', route => route.abort(), { times: 1 });
  await page.locator('#logout').click();
  await expect(page.locator('[data-oauth-provider="github"]')).toBeVisible();
});

test('a revoked Cookie session clears private data while retaining the OAuth login entry', async ({ page, service, request }) => {
  await signIn(page, service);
  const login = await request.post(`${service.url}/api/admin/login`, {
    data: { username: 'browser-admin', password: 'temporary-browser-password', id: 'test-admin', uuid: 'test-admin' },
  });
  const token = (await login.json()).access_token;
  const headers = { Authorization: `Bearer ${token}` };
  const sessions = await (await page.request.get(`${service.url}/api/admin/session/list`, { headers })).json();
  const ownSession = JSON.parse(Buffer.from(token.split('.')[1], 'base64url').toString()).jti;
  for (const session of sessions.data.filter(item => item.id !== ownSession)) {
    expect((await page.request.post(`${service.url}/api/admin/session/revoke`, { headers, data: { id: session.id } })).status()).toBe(200);
  }
  await page.locator('#nav-users').click();
  await expect(page.locator('#auth-form')).toBeVisible();
  await expect(page.locator('#nav-users')).toHaveCount(0);
  await expect(page.locator('[data-oauth-provider="github"]')).toBeVisible();
});
