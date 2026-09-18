import { defineConfig, devices } from '@playwright/test';

// Point BASE_URL at a deployed app (GitHub Pages, Cloudflare) to run the
// suite live; otherwise a static server serves the CSR build from ../examples.
const baseURL = process.env.BASE_URL ?? 'http://127.0.0.1:4173/';

export default defineConfig({
  testDir: './tests',
  timeout: 30_000,
  expect: { timeout: 10_000 },
  retries: process.env.CI ? 2 : 0,
  reporter: process.env.CI ? [['github'], ['list']] : 'list',
  use: { baseURL, trace: 'retain-on-failure' },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
  webServer: process.env.BASE_URL
    ? undefined
    : {
        command:
          'python3 -m http.server 4173 --bind 127.0.0.1 --directory ../examples/leptos-csr/dist',
        url: 'http://127.0.0.1:4173/',
        reuseExistingServer: !process.env.CI,
      },
});
