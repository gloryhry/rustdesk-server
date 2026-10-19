import { test, expect, signIn } from './fixtures.js';
import { DatabaseSync } from 'node:sqlite';
import { join } from 'node:path';
import { createHash } from 'node:crypto';

function groupedHex(bytes) {
  return bytes.toString('hex').match(/.{4}/g).join(' ');
}

async function addPeer(service, sqlite, pk = Buffer.alloc(32, 3)) {
  sqlite.prepare('insert into peer(guid,id,uuid,pk,info) values(?,?,?,?,?)').run(Buffer.alloc(16, 1), '123456', Buffer.from('device-uuid'), pk, '{}');
  const response = await fetch(`${service.url}/api/sysinfo`, {
    method: 'POST', headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ id: '123456', uuid: Buffer.from('device-uuid').toString('base64'), hostname: 'unsigned-laptop', username: 'untrusted' })
  });
  expect(await response.text()).toBe('SYSINFO_UPDATED');
}

test('administrator independently checks the device fingerprint before binding or unbinding', async ({ page, service, request }) => {
  const sqlite = new DatabaseSync(join(service.directory, 'api.sqlite3'));
  const pk = Buffer.alloc(32, 3);
  await addPeer(service, sqlite, pk);
  const expectedFingerprint = `sha256:${createHash('sha256').update(pk).digest('hex')}`;

  await signIn(page, service);
  await page.locator('#nav-devices').click();
  const peer = page.locator('[data-registry-peer="123456"]');
  await expect(peer).toContainText('unsigned-laptop');
  await expect(peer).toContainText(/待核验|pending verification/);
  await expect(page.locator('.content')).not.toContainText(expectedFingerprint);
  await expect(page.locator('.content')).not.toContainText(pk.toString('hex'));

  const account = sqlite.prepare("select id from api_user where username='browser-admin'").get().id;
  await peer.locator('[name="user_id"]').selectOption(account);
  await peer.locator('[name="pk_fingerprint"]').fill(groupedHex(Buffer.alloc(32, 4)));
  await expect(peer.locator('[data-fingerprint-status]')).toContainText(/不匹配|does not match/);
  await peer.locator('[type="submit"]').click();
  await expect(page.locator('.notice')).toContainText('device_binding_conflict');

  await peer.locator('[name="user_id"]').selectOption(account);
  await peer.locator('[name="pk_fingerprint"]').fill(`指纹: ${groupedHex(pk).toUpperCase()}`);
  await expect(peer.locator('[data-fingerprint-status]')).toContainText(/指纹匹配|Fingerprint matches/);
  const bindRequest = page.waitForRequest(request => request.url().endsWith('/api/admin/device/bind'));
  await peer.locator('[type="submit"]').click();
  expect((await bindRequest).postDataJSON().pk_fingerprint).toBe(expectedFingerprint);
  await expect(peer.locator('[data-unbind-device]')).toBeVisible();
  await expect(page.locator('.content .table-wrap table').first()).toContainText('123456');
  await peer.locator('[data-unbind-device]').click();
  await expect(peer.locator('[data-unbind-device]')).toHaveCount(0);
  expect(sqlite.prepare('select count(*) as n from api_device_binding_audit').get().n).toBe(2);
  sqlite.close();
});

test('sha256-prefixed fingerprints stay compatible and invalid input disables submission', async ({ page, service }) => {
  const sqlite = new DatabaseSync(join(service.directory, 'api.sqlite3'));
  const pk = Buffer.alloc(32, 5);
  await addPeer(service, sqlite, pk);
  const expectedFingerprint = `sha256:${createHash('sha256').update(pk).digest('hex')}`;
  await signIn(page, service);
  await page.locator('#nav-devices').click();
  const peer = page.locator('[data-registry-peer="123456"]');
  const account = sqlite.prepare("select id from api_user where username='browser-admin'").get().id;
  await peer.locator('[name="user_id"]').selectOption(account);

  await peer.locator('[name="pk_fingerprint"]').fill('1234');
  await expect(peer.locator('[data-fingerprint-status]')).toContainText(/64|sha256/);
  await expect(peer.locator('[type="submit"]')).toBeDisabled();

  await peer.locator('[name="pk_fingerprint"]').fill(expectedFingerprint.toUpperCase());
  await expect(peer.locator('[data-fingerprint-status]')).toContainText(/指纹匹配|Fingerprint matches/);
  const bindRequest = page.waitForRequest(request => request.url().endsWith('/api/admin/device/bind'));
  await peer.locator('[type="submit"]').click();
  expect((await bindRequest).postDataJSON().pk_fingerprint).toBe(expectedFingerprint);
  await expect(peer.locator('[data-unbind-device]')).toBeVisible();
  sqlite.close();
});
