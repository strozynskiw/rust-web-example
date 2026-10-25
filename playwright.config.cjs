const { defineConfig } = require('@playwright/test');

module.exports = defineConfig({
  testDir: './tests/e2e',
  workers: 1,
  use: { baseURL: 'http://localhost:3107', browserName: 'chromium' },
  webServer: {
    command: 'cargo run --locked --bin web-template',
    url: 'http://localhost:3107/healthz',
    reuseExistingServer: false,
    timeout: 120_000,
    env: {
      DATABASE_URL: 'sqlite::memory:', HOST: '127.0.0.1', PORT: '3107',
      APP_ORIGIN: 'http://localhost:3107', RUST_LOG: 'web_template=warn',
    },
  },
});
