import { test, expect, signIn } from './fixtures.js';
import { DatabaseSync } from 'node:sqlite';
import { join } from 'node:path';

test('editing an imported address book alias keeps password, RDP and unknown fields', async ({ page, service }) => {
  const sqlite = new DatabaseSync(join(service.directory, 'api.sqlite3'));
  const owner = sqlite.prepare("select id from api_user where username='browser-admin'").get().id;
  const peer = { id: '123456', alias: 'Original', hash: 'hash-value', password: 'password-value', rdpPort: '3390', rdpUsername: 'rdp-user', note: 'Saved note', extension: { keep: true }, tags: ['old'] };
  sqlite.prepare('insert into api_address_book_snapshot(user_id,data) values(?,?)').run(owner, JSON.stringify({ peers: [peer], tags: ['old'] }));
  await signIn(page, service);
  await page.locator('#nav-addressBook').click();
  await page.locator('[data-edit-peer]').click();
  await page.locator('[name="peer-alias"]').fill('Edited');
  await page.locator('#save-peer').click();
  await expect(page.locator('.table-wrap')).toContainText('Edited');
  const saved = JSON.parse(sqlite.prepare('select data from api_address_book_snapshot where user_id=?').get(owner).data).peers[0];
  for (const field of ['hash', 'password', 'rdpPort', 'rdpUsername', 'note', 'extension']) expect(saved[field]).toEqual(peer[field]);
  expect(saved.alias).toBe('Edited');
  await page.reload();
  await page.locator('#nav-addressBook').click();
  await expect(page.locator('.table-wrap')).toContainText('Edited');
  sqlite.close();
});

test('imported snapshot-only entry can be deleted and stays absent after reload', async ({ page, service }) => {
  const sqlite = new DatabaseSync(join(service.directory, 'api.sqlite3'));
  const owner = sqlite.prepare("select id from api_user where username='browser-admin'").get().id;
  sqlite.prepare('insert into api_address_book_snapshot(user_id,data) values(?,?)').run(owner, JSON.stringify({ peers: [{ id: '654321', alias: 'Imported only' }] }));
  await signIn(page, service);
  await page.locator('#nav-addressBook').click();
  await expect(page.locator('[data-delete-peer]')).toHaveCount(1);
  await page.locator('[data-delete-peer]').click();
  await expect(page.locator('[data-delete-peer]')).toHaveCount(0);
  expect(JSON.parse(sqlite.prepare('select data from api_address_book_snapshot where user_id=?').get(owner).data).peers).toEqual([]);
  await page.reload();
  await page.locator('#nav-addressBook').click();
  await expect(page.locator('[data-delete-peer]')).toHaveCount(0);
  sqlite.close();
});

test('Peer ID edit preserves stable entry ID and duplicate targets leave data unchanged', async ({ page, service }) => {
  const sqlite = new DatabaseSync(join(service.directory, 'api.sqlite3'));
  const owner = sqlite.prepare("select id from api_user where username='browser-admin'").get().id;
  const peers = [{ id: '123456', entryId: 'stable-first', alias: 'First', password: 'saved-password', extension: { kept: true } }, { id: '654321', entryId: 'stable-second', alias: 'Second' }];
  sqlite.prepare('insert into api_address_book_snapshot(user_id,data) values(?,?)').run(owner, JSON.stringify({ peers }));
  const insert = sqlite.prepare('insert into api_address_book_entry(id,user_id,peer_id,alias) values(?,?,?,?)');
  for (const peer of peers) insert.run(peer.entryId, owner, peer.id, peer.alias);
  await signIn(page, service);
  await page.locator('#nav-addressBook').click();
  await page.locator('[data-edit-peer="stable-first"]').click();
  await page.locator('[name="peer-id"]').fill('777777');
  await page.locator('#save-peer').click();
  await expect(page.locator('.table-wrap')).toContainText('777777');
  expect(sqlite.prepare("select peer_id from api_address_book_entry where id='stable-first'").get().peer_id).toBe('777777');
  const before = sqlite.prepare('select data from api_address_book_snapshot where user_id=?').get(owner).data;
  const first = JSON.parse(before).peers.find(peer => peer.id === '777777');
  expect(first.entryId).toBe('stable-first'); expect(first.password).toBe('saved-password'); expect(first.extension).toEqual({ kept: true });
  await page.locator('[data-edit-peer="stable-first"]').click();
  await page.locator('[name="peer-id"]').fill('654321');
  await page.locator('#save-peer').click();
  await expect(page.locator('.notice')).toContainText('address_book_entry_conflict');
  expect(sqlite.prepare('select data from api_address_book_snapshot where user_id=?').get(owner).data).toBe(before);
  sqlite.close();
});
