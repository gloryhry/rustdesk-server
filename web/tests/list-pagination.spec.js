import { test, expect, signIn } from './fixtures.js';
import { DatabaseSync } from 'node:sqlite';
import { join } from 'node:path';

test('group account selector reads users beyond the first official page', async ({ page, service }) => {
  const sqlite = new DatabaseSync(join(service.directory, 'api.sqlite3'));
  const admin = sqlite.prepare("select id from api_user where username='browser-admin'").get().id;
  const insert = sqlite.prepare("insert into api_user(id,username,password_hash) values(?,?,'unused')");
  for (let i = 0; i < 101; i += 1) insert.run(`page-user-${i}`, `user-${String(i).padStart(3, '0')}`);
  sqlite.prepare('insert into api_user_group(id,name,created_by) values(?,?,?)').run('paged-group', 'All pages', admin);
  await signIn(page, service);
  await page.locator('#nav-groups').click();
  const selector = page.locator('#group-select-paged-group');
  await expect(selector.locator('option')).toHaveCount(102);
  await selector.selectOption('page-user-100');
  await page.locator('[data-add-group="paged-group"]').click();
  await expect(page.locator('.membership-list')).toContainText('user-100');
  expect(sqlite.prepare("select user_id from api_user_group_member where group_id='paged-group'").get().user_id).toBe('page-user-100');
  sqlite.close();
});
