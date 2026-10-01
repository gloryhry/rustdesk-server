import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './tests',
  timeout: 30000,
  workers: 1,
  use: {
    headless: true,
    launchOptions: { executablePath: process.env.PLAYWRIGHT_CHROME || '/usr/bin/google-chrome' },
  },
  outputDir: 'test-results',
});
