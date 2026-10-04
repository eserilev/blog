// Browser tests (spec 7.5). One server for all tests, with a fresh database.
import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './tests',
  fullyParallel: false,
  workers: 1,
  retries: 0,
  reporter: [['list']],
  use: {
    baseURL: 'http://localhost:18100',
    trace: 'retain-on-failure',
  },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
  webServer: {
    command: './serve.sh',
    url: 'http://localhost:18100/healthz',
    timeout: 600_000,
    reuseExistingServer: false,
  },
});
