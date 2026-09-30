import { test, expect, signIn } from './fixtures.js';
import { DatabaseSync } from 'node:sqlite';
import { join } from 'node:path';

test('whole document editor reloads canonical identities and revisions before further edits', async ({ page, service }) => {
  await signIn(page, service); await page.locator('#nav-addressBook').click();
  await page.locator('#address-book-editor').fill(JSON.stringify({ peers: [{ id: '123456', alias: 'JSON import', extension: { keep: true } }], tags: ['imported'], tag_colors: { imported: 0xff112233 } }));
  await page.locator('#save-address-book').click(); await expect(page.locator('[data-edit-peer]')).toHaveCount(1);
  await expect(page.locator('[data-edit-tag="imported"]')).toBeVisible();
  const canonical = JSON.parse(await page.locator('#address-book-editor').inputValue()); expect(canonical.peers[0].entryId).toBeTruthy();
  await page.locator('[data-edit-peer]').click(); await page.locator('[name="peer-alias"]').fill('After JSON'); await page.locator('#save-peer').click();
  await expect(page.locator('.table-wrap')).toContainText('After JSON');
  const saved = await page.evaluate(async () => JSON.parse((await (await fetch('/api/ab')).json()).data));
  expect(saved.peers[0].extension).toEqual({ keep: true }); expect(saved.peers[0].entryId).toBe(canonical.peers[0].entryId);
});

test('two tabs preserve all peers and reject a stale draft even after list refresh', async ({ page, service, context }) => {
  const sqlite = new DatabaseSync(join(service.directory, 'api.sqlite3'));
  const owner = sqlite.prepare("select id from api_user where username='browser-admin'").get().id;
  sqlite.prepare('insert into api_address_book_snapshot(user_id,data) values(?,?)').run(owner, JSON.stringify({ peers: [{ id: '123456', alias: 'First' }, { id: '654321', alias: 'Second', password: 'keep' }] }));
  await signIn(page, service); await page.locator('#nav-addressBook').click();
  const second = await context.newPage(); await second.goto(service.uiUrl); await second.locator('#nav-addressBook').click();
  await second.locator('[data-edit-peer]').first().click(); await second.locator('[name="peer-alias"]').fill('Stale draft');
  await page.locator('[data-edit-peer]').first().click(); await page.locator('[name="peer-alias"]').fill('Fresh'); await page.locator('#save-peer').click();
  await expect(page.locator('.table-wrap')).toContainText('Fresh'); await expect(page.locator('[data-edit-peer]')).toHaveCount(2);
  await second.locator('#refresh-address-book').click(); await expect(second.locator('.table-wrap')).toContainText('Fresh');
  await second.locator('#save-peer').click(); await expect(second.locator('.notice')).toContainText('address_book_revision_conflict');
  let document = JSON.parse(sqlite.prepare('select data from api_address_book_snapshot where user_id=?').get(owner).data);
  expect(document.peers[0].alias).toBe('Fresh'); expect(document.peers[1].password).toBe('keep');
  await second.locator('[data-edit-peer]').first().click(); await second.locator('[name="peer-alias"]').fill('Recovered'); await second.locator('#save-peer').click();
  await expect(second.locator('.table-wrap')).toContainText('Recovered');
  document = JSON.parse(sqlite.prepare('select data from api_address_book_snapshot where user_id=?').get(owner).data);
  expect(document.peers).toHaveLength(2); await second.close(); sqlite.close();
});

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

test('official false string restores an unchecked relay and Web edits round-trip both values', async ({ page, service }) => {
  const sqlite = new DatabaseSync(join(service.directory, 'api.sqlite3'));
  const owner = sqlite.prepare("select id from api_user where username='browser-admin'").get().id;
  sqlite.prepare('insert into api_address_book_snapshot(user_id,data) values(?,?)').run(owner, JSON.stringify({ peers: [{ id: '123456', forceAlwaysRelay: 'false' }] }));
  await signIn(page, service);
  await page.locator('#nav-addressBook').click();
  await page.locator('[data-edit-peer]').click();
  const relay = page.locator('[name="peer-relay"]');
  await expect(relay).not.toBeChecked();
  await relay.check(); await page.locator('#save-peer').click();
  await expect(page.locator('[data-edit-peer]')).toHaveCount(1);
  await page.locator('[data-edit-peer]').click(); await expect(relay).toBeChecked();
  let official = await page.evaluate(async () => JSON.parse((await (await fetch('/api/ab')).json()).data));
  expect(official.peers[0].forceAlwaysRelay).toBe('true');
  await relay.uncheck(); await page.locator('#save-peer').click();
  await expect(page.locator('[data-edit-peer]')).toHaveCount(1);
  await page.reload(); await page.locator('#nav-addressBook').click(); await page.locator('[data-edit-peer]').click();
  await expect(relay).not.toBeChecked();
  official = await page.evaluate(async () => JSON.parse((await (await fetch('/api/ab')).json()).data));
  expect(official.peers[0].forceAlwaysRelay).toBe('false');
  expect(JSON.parse(sqlite.prepare('select data from api_address_book_snapshot where user_id=?').get(owner).data).peers[0].forceAlwaysRelay).toBe(false);
  sqlite.close();
});

test('tag color editing keeps alpha and renaming or deleting updates only related Peer references', async ({ page, service }) => {
  const sqlite = new DatabaseSync(join(service.directory, 'api.sqlite3'));
  const owner = sqlite.prepare("select id from api_user where username='browser-admin'").get().id;
  const initial = { tags: ['old', 'keep'], tag_colors: JSON.stringify({ old: 0x44112233, keep: 0xffabcdef }), peers: [{ id: '123456', entryId: 'stable-peer', tags: ['old', 'keep'] }] };
  sqlite.prepare('insert into api_address_book_snapshot(user_id,data) values(?,?)').run(owner, JSON.stringify(initial));
  sqlite.prepare('insert into api_address_book_entry(id,user_id,peer_id,tags) values(?,?,?,?)').run('stable-peer', owner, '123456', JSON.stringify(['old', 'keep']));
  await signIn(page, service); await page.locator('#nav-addressBook').click();
  await page.locator('[data-edit-tag="old"]').click();
  await expect(page.locator('#tag-form [name="color"]')).toHaveValue('#11223344');
  await page.locator('#tag-form [name="name"]').fill('renamed');
  await page.locator('#tag-form [name="color"]').fill('#34567812');
  await page.locator('#save-tag').click();
  await expect(page.locator('[data-edit-tag="renamed"]')).toBeVisible();
  let stored = JSON.parse(sqlite.prepare('select data from api_address_book_snapshot where user_id=?').get(owner).data);
  expect(stored.tag_colors).toEqual({ renamed: 0x12345678, keep: 0xffabcdef });
  expect(stored.peers[0].tags).toEqual(['renamed', 'keep']);
  expect(JSON.parse(sqlite.prepare("select tags from api_address_book_entry where id='stable-peer'").get().tags)).toEqual(['renamed', 'keep']);
  await page.locator('[data-delete-tag="renamed"]').click();
  await expect(page.locator('[data-delete-tag="renamed"]')).toHaveCount(0);
  stored = JSON.parse(sqlite.prepare('select data from api_address_book_snapshot where user_id=?').get(owner).data);
  expect(stored.tag_colors).toEqual({ keep: 0xffabcdef }); expect(stored.peers[0].tags).toEqual(['keep']);
  await page.reload(); await page.locator('#nav-addressBook').click();
  await expect(page.locator('[data-edit-tag="keep"]')).toBeVisible();
  sqlite.close();
});
