import { test, expect, signIn } from './fixtures.js';
import { DatabaseSync } from 'node:sqlite';
import { join } from 'node:path';

test('deleting a stable entry does not delete a peer whose ID collides with that entry', async ({ page, service }) => {
  await signIn(page, service);
  await page.locator('#nav-addressBook').click();
  await page.locator('#address-book-editor').fill(JSON.stringify({ peers: [{ id: '111111', alias: 'FIRST', password: 'keep' }, { id: '222222', alias: 'SECOND' }] }));
  await page.locator('#save-address-book').click();
  await expect(page.locator('[data-edit-peer]')).toHaveCount(2);
  const canonical = JSON.parse(await page.locator('#address-book-editor').inputValue());
  await page.locator('[data-edit-peer]').first().click();
  await page.locator('[name="peer-id"]').fill(canonical.peers[1].entryId);
  await page.locator('#save-peer').click();
  await expect(page.locator('[data-edit-peer]')).toHaveCount(2);
  await page.locator('tr').filter({ hasText: 'SECOND' }).locator('[data-delete-peer]').click();
  await expect(page.locator('[data-edit-peer]')).toHaveCount(1);
  await expect(page.locator('.table-wrap')).toContainText('FIRST');
  await page.reload();
  await page.locator('#nav-addressBook').click();
  await expect(page.locator('[data-edit-peer]')).toHaveCount(1);
  await expect(page.locator('.table-wrap')).toContainText('FIRST');
  const saved = JSON.parse(await page.locator('#address-book-editor').inputValue());
  expect(saved.peers[0].password).toBe('keep');
});

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
  const enabled = page.waitForResponse(response => response.url().endsWith('/api/web/ab/entries') && response.request().method() === 'POST');
  await relay.check(); await page.locator('#save-peer').click();
  expect((await enabled).status()).toBe(200);
  await expect(page.locator('[data-edit-peer]')).toHaveCount(1);
  await page.locator('[data-edit-peer]').click(); await expect(relay).toBeChecked();
  let official = await page.evaluate(async () => JSON.parse((await (await fetch('/api/ab')).json()).data));
  expect(official.peers[0].forceAlwaysRelay).toBe('true');
  const disabled = page.waitForResponse(response => response.url().endsWith('/api/web/ab/entries') && response.request().method() === 'POST');
  await relay.uncheck(); await page.locator('#save-peer').click();
  expect((await disabled).status()).toBe(200);
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


test('refreshing the book must refresh tag colors before assigning the new revision', async ({ page, service, context }) => {
  await signIn(page, service);
  await page.locator('#nav-addressBook').click();
  await page.locator('#tag-form [name="name"]').fill('work');
  await page.locator('#tag-form [name="color"]').fill('#ff0000');
  await page.locator('#save-tag').click();
  await expect(page.locator('[data-edit-tag="work"]')).toBeVisible();
  await expect(page.locator('#save-tag')).toBeEnabled();
  const second = await context.newPage();
  await second.goto(service.uiUrl);
  await second.locator('#nav-addressBook').click();
  await second.locator('[data-edit-tag="work"]').click();
  await second.locator('#tag-form [name="color"]').fill('#0000ff');
  const secondSaved = second.waitForResponse(r => r.url().endsWith('/api/web/ab/tags') && r.request().method() === 'POST');
  await second.locator('#save-tag').click();
  expect((await secondSaved).status()).toBe(200);
  await expect(second.locator('#save-tag')).toBeEnabled();
  await page.locator('#refresh-address-book').click();
  await expect.poll(async () => JSON.parse(await page.locator('#address-book-editor').inputValue()).tag_colors).toEqual('{"work":4278190335}');
  await page.locator('[data-edit-tag="work"]').click();
  const displayed = await page.locator('#tag-form [name="color"]').inputValue();
  await page.locator('#tag-form [name="name"]').fill('renamed');
  const response = page.waitForResponse(r => r.url().endsWith('/api/web/ab/tags') && r.request().method() === 'POST');
  await page.locator('#save-tag').click();
  const status = (await response).status();
  const stored = await page.evaluate(async () => JSON.parse((await (await fetch('/api/ab')).json()).data));
  expect(displayed).toBe('#0000ff');
  expect(status).toBe(200);
  expect(JSON.parse(stored.tag_colors).renamed).toBe(0xff0000ff);
});

test('saving a tag must keep the unsaved whole document draft', async ({ page, service }) => {
  await signIn(page, service);
  await page.locator('#nav-addressBook').click();
  const draft = JSON.stringify({peers:[{id:'123456',alias:'unsaved-draft'}],tags:[]});
  await page.locator('#address-book-editor').fill(draft);
  await page.locator('#tag-form [name="name"]').fill('new-tag');
  await page.locator('#save-tag').click();
  await expect(page.locator('[data-edit-tag="new-tag"]')).toBeVisible();
  const after = JSON.parse(await page.locator('#address-book-editor').inputValue());
  await expect(page.locator('#address-book-editor')).toHaveValue(draft);
  expect(after.peers).toEqual([{id:'123456',alias:'unsaved-draft'}]);
});

async function openAddressBook(page, service) {
  await signIn(page, service);
  await page.locator('#nav-addressBook').click();
  await expect(page.locator('#save-tag')).toBeEnabled();
}

async function saveJsonBook(page, document) {
  await page.locator('#address-book-editor').fill(JSON.stringify(document));
  await page.locator('#save-address-book').click();
  await expect(page.locator('#save-address-book')).toBeEnabled();
}

test('a tag draft retains its revision after refresh and rejects a concurrent edit', async ({ page, service, context }) => {
  await openAddressBook(page, service);
  await saveJsonBook(page, { peers: [], tags: ['work'], tag_colors: { work: 0xffff0000 } });
  await page.locator('[data-edit-tag="work"]').click();
  await page.locator('#tag-form [name="name"]').fill('stale-name');
  const second = await context.newPage();
  await second.goto(service.uiUrl);
  await second.locator('#nav-addressBook').click();
  await expect(second.locator('#save-tag')).toBeEnabled();
  await second.locator('[data-edit-tag="work"]').click();
  await second.locator('#tag-form [name="color"]').fill('#0000ff');
  await second.locator('#save-tag').click();
  await expect(second.locator('#save-tag')).toBeEnabled();
  await page.locator('#refresh-address-book').click();
  await expect(page.locator('#save-tag')).toBeEnabled();
  const response = page.waitForResponse(r => r.url().endsWith('/api/web/ab/tags') && r.request().method() === 'POST');
  await page.locator('#save-tag').click();
  expect((await response).status()).toBe(409);
  await expect(page.locator('.notice')).toContainText('address_book_revision_conflict');
  await expect(page.locator('#tag-form [name="name"]')).toHaveValue('stale-name');
  const stored = await page.evaluate(async () => JSON.parse((await (await fetch('/api/ab')).json()).data));
  expect(JSON.parse(stored.tag_colors)).toEqual({ work: 0xff0000ff });
  expect(stored.tags).toEqual(['work']);
});

for (const action of ['create', 'edit', 'delete']) {
  test(`a ${action} tag action preserves exact JSON and its original revision`, async ({ page, service }) => {
    await openAddressBook(page, service);
    await saveJsonBook(page, { peers: [], tags: ['work'], tag_colors: { work: 0xffff0000 } });
    const initial = await page.evaluate(async () => (await fetch('/api/ab')).json());
    const draft = ' {\n "peers": [{"id":"123456", "extension":{"keep":true}}], "tags": ["work"]\n} ';
    await page.locator('#address-book-editor').fill(draft);
    if (action === 'delete') {
      await page.locator('[data-delete-tag="work"]').click();
    } else {
      if (action === 'edit') await page.locator('[data-edit-tag="work"]').click();
      await page.locator('#tag-form [name="name"]').fill('new-name');
      await page.locator('#save-tag').click();
    }
    await expect(page.locator('#save-tag')).toBeEnabled();
    await expect(page.locator('#address-book-editor')).toHaveValue(draft);
    const before = await page.evaluate(async () => (await fetch('/api/ab')).json());
    expect(before.revision).toBeGreaterThan(initial.revision);
    const response = page.waitForResponse(r => r.url().endsWith('/api/ab') && r.request().method() === 'POST');
    await page.locator('#save-address-book').click();
    const rejected = await response;
    expect(rejected.request().postDataJSON().revision).toBe(initial.revision);
    expect(rejected.status()).toBe(409);
    await expect(page.locator('#address-book-editor')).toHaveValue(draft);
    expect(await page.evaluate(async () => (await fetch('/api/ab')).json())).toEqual(before);
  });
}

test('invalid JSON survives language, navigation, failed refresh and invalid save', async ({ page, service }) => {
  await openAddressBook(page, service);
  const draft = ' { "peers": [\n   incomplete';
  await page.locator('#address-book-editor').fill(draft);
  await page.locator('#locale-toggle').click();
  await expect(page.locator('#address-book-editor')).toHaveValue(draft);
  await expect(page.locator('#address-book-draft-status')).toContainText('unsaved');
  await page.locator('#nav-profile').click();
  await page.locator('#nav-addressBook').click();
  await expect(page.locator('#save-address-book')).toBeEnabled();
  await expect(page.locator('#address-book-editor')).toHaveValue(draft);
  await page.route('**/api/web/ab/tags', route => route.fulfill({ status: 503, json: { error: 'review_refresh_failure' } }));
  await page.locator('#refresh-address-book').click();
  await expect(page.locator('.notice')).toContainText('review_refresh_failure');
  await expect(page.locator('#address-book-editor')).toHaveValue(draft);
  await expect(page.locator('#save-tag')).toBeDisabled();
  await page.unroute('**/api/web/ab/tags');
  await page.locator('#refresh-address-book').click();
  await expect(page.locator('#save-address-book')).toBeEnabled();
  await page.locator('#save-address-book').click();
  await expect(page.locator('.notice')).toContainText('valid JSON');
  await expect(page.locator('#address-book-editor')).toHaveValue(draft);
});

test('discarding JSON requires confirmation and a successful snapshot read', async ({ page, service }) => {
  await openAddressBook(page, service);
  const original = await page.locator('#address-book-editor').inputValue();
  const draft = '{ "unsaved": true';
  await page.locator('#address-book-editor').fill(draft);
  page.once('dialog', dialog => dialog.dismiss());
  await page.locator('#discard-address-book-draft').click();
  await expect(page.locator('#address-book-editor')).toHaveValue(draft);
  await page.route('**/api/web/ab/tags', route => route.fulfill({ status: 503, json: { error: 'discard_read_failure' } }));
  page.once('dialog', dialog => dialog.accept());
  await page.locator('#discard-address-book-draft').click();
  await expect(page.locator('.notice')).toContainText('discard_read_failure');
  await expect(page.locator('#address-book-editor')).toHaveValue(draft);
  await page.unroute('**/api/web/ab/tags');
  page.once('dialog', dialog => dialog.accept());
  await page.locator('#discard-address-book-draft').click();
  await expect(page.locator('#save-address-book')).toBeEnabled();
  await expect(page.locator('#address-book-editor')).toHaveValue(original);
  await expect(page.locator('#discard-address-book-draft')).toHaveCount(0);
});

for (const persistent of [false, true]) {
  test(`a ${persistent ? 'persistent' : 'transient'} snapshot mismatch never mixes tag data and revisions`, async ({ page, service, context }) => {
    await openAddressBook(page, service);
    await saveJsonBook(page, { peers: [], tags: ['work'], tag_colors: { work: 0xffff0000 } });
    const original = await page.locator('#address-book-editor').inputValue();
    const second = await context.newPage();
    await second.goto(service.uiUrl);
    await second.locator('#nav-addressBook').click();
    await expect(second.locator('#save-tag')).toBeEnabled();
    await second.locator('[data-edit-tag="work"]').click();
    await second.locator('#tag-form [name="color"]').fill('#0000ff');
    await second.locator('#save-tag').click();
    await expect(second.locator('#save-tag')).toBeEnabled();
    let reads = 0;
    await page.route('**/api/web/ab/tags', async route => {
      const response = await route.fetch();
      const body = await response.json();
      reads += 1;
      if (persistent || reads === 1) body.revision += 1;
      await route.fulfill({ response, json: body });
    });
    await page.locator('#refresh-address-book').click();
    await expect.poll(() => reads).toBe(2);
    if (persistent) {
      await expect(page.locator('.notice')).toContainText('address_book_revision_conflict');
      await expect(page.locator('#address-book-editor')).toHaveValue(original);
      await expect(page.locator('[data-edit-tag="work"]')).toBeDisabled();
      await expect(page.locator('.tag-swatch')).toHaveAttribute('title', '#ff0000');
    } else {
      await expect(page.locator('#save-tag')).toBeEnabled();
      await page.locator('[data-edit-tag="work"]').click();
      await expect(page.locator('#tag-form [name="color"]')).toHaveValue('#0000ff');
    }
  });
}

test('a successful write followed by failed refresh blocks writes and keeps the JSON draft', async ({ page, service }) => {
  await openAddressBook(page, service);
  const draft = '{ "pending": true';
  await page.locator('#address-book-editor').fill(draft);
  await page.locator('#tag-form [name="name"]').fill('saved-tag');
  await page.route('**/api/web/ab/tags', route => route.request().method() === 'GET'
    ? route.fulfill({ status: 503, json: { error: 'after_write_failure' } }) : route.continue());
  const response = page.waitForResponse(r => r.url().endsWith('/api/web/ab/tags') && r.request().method() === 'POST');
  await page.locator('#save-tag').click();
  expect((await response).status()).toBe(200);
  await expect(page.locator('.notice')).toContainText('已保存，但刷新失败');
  await expect(page.locator('#save-tag')).toBeDisabled();
  await expect(page.locator('#save-peer')).toBeDisabled();
  await expect(page.locator('#save-address-book')).toBeDisabled();
  await expect(page.locator('#refresh-address-book')).toBeEnabled();
  await expect(page.locator('#address-book-editor')).toHaveValue(draft);
  await page.unroute('**/api/web/ab/tags');
  await page.locator('#refresh-address-book').click();
  await expect(page.locator('[data-edit-tag="saved-tag"]')).toBeEnabled();
  await expect(page.locator('#address-book-editor')).toHaveValue(draft);
});

test('a snapshot returning after logout cannot restore private drafts', async ({ page, service }) => {
  await openAddressBook(page, service);
  await saveJsonBook(page, { peers: [{ id: '123456', alias: 'private-snapshot' }], tags: ['private-tag'] });
  await page.locator('#address-book-editor').fill('{ "privateDraft": true');
  let release;
  const gate = new Promise(resolve => { release = resolve; });
  let received;
  const ready = new Promise(resolve => { received = resolve; });
  await page.route('**/api/web/ab/tags', async route => {
    const response = await route.fetch();
    received();
    await gate;
    await route.fulfill({ response });
  });
  await page.locator('#refresh-address-book').click();
  await ready;
  await page.locator('#logout').click();
  await expect(page.locator('#username')).toBeVisible();
  release();
  await page.unrouteAll({ behavior: 'wait' });
  await expect(page.locator('#address-book-editor')).toHaveCount(0);
  await signIn(page, service);
  await page.locator('#nav-addressBook').click();
  await expect(page.locator('#save-tag')).toBeEnabled();
  await expect(page.locator('#address-book-editor')).not.toHaveValue('{ "privateDraft": true');
});

test('a late unauthorized snapshot cannot clear a newly authenticated session', async ({ page, service }) => {
  await openAddressBook(page, service);
  let release;
  const gate = new Promise(resolve => { release = resolve; });
  let received;
  const ready = new Promise(resolve => { received = resolve; });
  let held = false;
  await page.route('**/api/web/ab/tags', async route => {
    if (held) { await route.continue(); return; }
    held = true;
    received();
    await gate;
    await route.fulfill({ status: 401, json: { error: 'old_session_expired' } });
  });
  await page.locator('#refresh-address-book').click();
  await ready;
  await page.locator('#logout').click();
  await expect(page.locator('#username')).toBeVisible();
  await page.locator('#username').fill('browser-admin');
  await page.locator('#password').fill('temporary-browser-password');
  await page.locator('#auth-submit').click();
  await expect(page.locator('#nav-addressBook')).toBeVisible();
  await page.locator('#nav-addressBook').click();
  await expect(page.locator('#save-tag')).toBeEnabled();
  const current = await page.locator('#address-book-editor').inputValue();
  release();
  await page.unrouteAll({ behavior: 'wait' });
  await expect(page.locator('#nav-addressBook')).toBeVisible();
  await expect(page.locator('#address-book-editor')).toHaveValue(current);
});

test('an unavailable bootstrap snapshot preserves the authenticated session', async ({ page, service }) => {
  await page.route('**/api/web/ab/tags', route => route.fulfill({ status: 503, json: { error: 'bootstrap_read_failure' } }));
  await signIn(page, service);
  await page.locator('#nav-addressBook').click();
  await expect(page.locator('.notice')).toContainText('bootstrap_read_failure');
  await expect(page.locator('#save-tag')).toBeDisabled();
  await expect(page.locator('#logout')).toBeVisible();
  await page.unroute('**/api/web/ab/tags');
  await page.locator('#refresh-address-book').click();
  await expect(page.locator('#save-tag')).toBeEnabled();
});

test('an unauthorized current snapshot immediately clears the private view', async ({ page, service }) => {
  await openAddressBook(page, service);
  await page.locator('#address-book-editor').fill('{ "privateDraft": true');
  await page.route('**/api/web/ab/tags', route => route.fulfill({ status: 401, json: { error: 'session_expired' } }));
  await page.locator('#refresh-address-book').click();
  await expect(page.locator('#username')).toBeVisible();
  await expect(page.locator('#address-book-editor')).toHaveCount(0);
  await expect(page.locator('#logout')).toHaveCount(0);
});
