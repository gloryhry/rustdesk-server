import { test, expect, signIn } from './fixtures.js';

const views = [
  ['addressBook', '#address-book-list', '#refresh-address-book'],
  ['devices', '#device-tbody', '#refresh-devices'],
  ['oauth', '#oauth-provider-form', '#refresh-oauth'],
  ['ldap', '#ldap-form', '#refresh-ldap'],
  ['profile', '.detail-grid', null]
];

async function expectAligned(page, readySelector) {
  await expect(page.locator(readySelector)).toBeVisible();
  const view = page.locator('.view');
  const header = page.locator('.page-header');
  const viewBox = await view.boundingBox();
  const headerBox = await header.boundingBox();
  expect(viewBox).not.toBeNull();
  expect(headerBox).not.toBeNull();
  expect(Math.abs(viewBox.x - headerBox.x)).toBeLessThan(1.5);
  expect(Math.abs(viewBox.width - headerBox.width)).toBeLessThan(1.5);

  const cards = page.locator('.view > .card');
  for (let index = 0; index < await cards.count(); index += 1) {
    const box = await cards.nth(index).boundingBox();
    expect(box).not.toBeNull();
    expect(Math.abs(box.x - viewBox.x)).toBeLessThan(1.5);
    expect(Math.abs(box.width - viewBox.width)).toBeLessThan(1.5);
  }
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
  expect(overflow).toBeLessThanOrEqual(1);
}

for (const [width, height] of [[1440, 900], [390, 844]]) {
  test(`admin pages align without horizontal overflow at ${width}px`, async ({ page, service }) => {
    await page.setViewportSize({ width, height });
    await signIn(page, service);
    for (const [view, ready, refresh] of views) {
      await page.locator(`#nav-${view}`).click();
      if (refresh) await expect(page.locator(refresh)).toBeEnabled();
      await expectAligned(page, ready);
    }
  });
}
