import { test, expect, signIn } from './fixtures.js';

async function openTab(page, name) {
  await page.locator(`#ldap-tabs [data-tab="${name}"]`).click();
  await expect(page.locator(`[data-ldap-panel="${name}"]`)).toBeVisible();
}

test('administrator edits LDAP connection, bind, and search settings across tabs', async ({ page, service }) => {
  await signIn(page, service);
  await page.locator('#nav-ldap').click();

  await page.locator('#ldap-url').fill('ldaps://directory.example');
  await page.locator('#ldap-timeout').fill('9');
  await openTab(page, 'bind');
  await page.locator('#ldap-bind-dn').fill('cn=admin,dc=example,dc=com');
  await page.locator('#ldap-bind-password').fill('directory-secret');
  await openTab(page, 'search');
  await page.locator('#ldap-user-base-dn').fill('ou=people,dc=example,dc=com');
  await page.locator('#ldap-user-filter').fill('(&(objectClass=person)(uid={username}))');
  await page.locator('#ldap-username-attribute').fill('uid');
  await page.locator('#ldap-email-attribute').fill('mail');

  const saved = page.waitForRequest(request => request.url().endsWith('/api/admin/ldap/config') && request.method() === 'POST');
  await page.locator('#save-ldap').click();
  const payload = (await saved).postDataJSON();
  expect(payload).toMatchObject({
    enabled: false,
    url: 'ldaps://directory.example',
    bind_dn: 'cn=admin,dc=example,dc=com',
    bind_password: 'directory-secret',
    user_base_dn: 'ou=people,dc=example,dc=com',
    user_filter: '(&(objectClass=person)(uid={username}))',
    username_attribute: 'uid',
    email_attribute: 'mail',
    use_tls: false,
    timeout_seconds: 9
  });
  await expect(page.locator('.notice')).toContainText(/LDAP 配置已应用|LDAP configuration applied/);

  await page.reload();
  await page.locator('#nav-ldap').click();
  await expect(page.locator('#ldap-url')).toHaveValue('ldaps://directory.example');
  await expect(page.locator('#ldap-timeout')).toHaveValue('9');
  await openTab(page, 'bind');
  await expect(page.locator('#ldap-bind-dn')).toHaveValue('cn=admin,dc=example,dc=com');
  await expect(page.locator('#ldap-bind-password')).toHaveValue('');
  await openTab(page, 'search');
  await expect(page.locator('#ldap-user-base-dn')).toHaveValue('ou=people,dc=example,dc=com');
  await expect(page.locator('#ldap-user-filter')).toHaveValue('(&(objectClass=person)(uid={username}))');
});
