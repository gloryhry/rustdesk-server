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
