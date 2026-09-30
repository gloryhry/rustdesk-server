import { test as base, expect } from '@playwright/test';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { spawn } from 'node:child_process';
import { createServer } from 'node:net';
import { once } from 'node:events';
import { randomBytes } from 'node:crypto';

export { expect };
export const test = base.extend({
  service: async ({}, use) => {
    const directory = await mkdtemp(join(tmpdir(), 'rustdesk-web-'));
    const listener = createServer();
    listener.listen(0, '127.0.0.1');
    await once(listener, 'listening');
    const port = listener.address().port;
    await new Promise(resolve => listener.close(resolve));
    const url = `http://127.0.0.1:${port}`;
    const env = Object.fromEntries(Object.entries(process.env).filter(([key]) =>
      !/^(API[_-]|RUSTDESK[_-]|DB[_-]URL)/i.test(key)));
    Object.assign(env, {
      API_ENABLED: '1', API_PORT: String(port), API_BIND: '127.0.0.1',
      API_PUBLIC_URL: url, API_ALLOW_INSECURE_LOCAL_HTTP: '1',
      API_JWT_SECRET: randomBytes(32).toString('hex'),
      API_OAUTH_CONFIG_KEY: randomBytes(32).toString('base64'),
      API_BOOTSTRAP_ADMIN_USERNAME: 'browser-admin',
      API_BOOTSTRAP_ADMIN_PASSWORD: 'temporary-browser-password',
      API_OAUTH_REDIRECT_URL: `${url}/api/oidc/callback`,
      API_GITHUB_CLIENT_ID: 'test-only', API_GITHUB_CLIENT_SECRET: 'test-only',
      DB_URL: join(directory, 'api.sqlite3'),
      RUSTDESK_KEY: Buffer.alloc(32, 1).toString('base64'),
      API_WEB_ROOT: resolve('dist'),
    });
    const binary = process.env.RUSTDESK_API_BINARY || resolve('../target/debug/rustdesk-api');
    const child = spawn(binary, [], { cwd: directory, env, stdio: ['ignore', 'pipe', 'pipe'] });
    let output = '';
    child.stdout.on('data', bytes => { output += bytes; });
    child.stderr.on('data', bytes => { output += bytes; });
    const exited = once(child, 'exit');
    try {
      await expect.poll(async () => {
        if (child.exitCode !== null) throw new Error(`isolated API exited: ${output}`);
        try { return (await fetch(`${url}/health/live`)).status; } catch { return 0; }
      }, { timeout: 15000 }).toBe(200);
      await use({ url, directory });
    } finally {
      if (child.exitCode === null) child.kill('SIGTERM');
      await exited;
      await rm(directory, { recursive: true, force: true });
    }
  },
});

export async function signIn(page, service) {
  await page.goto(service.url);
  await page.locator('#username').fill('browser-admin');
  await page.locator('#password').fill('temporary-browser-password');
  await page.locator('#auth-submit').click();
  await expect(page.locator('#nav-oauth')).toBeVisible();
}
