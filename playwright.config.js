const { defineConfig } = require("@playwright/test");

const port = Number(process.env.UI_PORT || 8050);

module.exports = defineConfig({
  testDir: "./ui/tests",
  timeout: 15_000,
  expect: { timeout: 5_000 },
  fullyParallel: false,
  workers: 1,
  reporter: "line",
  use: {
    baseURL: `http://127.0.0.1:${port}`,
    browserName: "chromium",
    trace: "retain-on-failure",
    screenshot: "only-on-failure"
  },
  webServer: {
    command: `python3 ui/serve.py --port ${port} --bind 127.0.0.1`,
    url: `http://127.0.0.1:${port}/#/entry`,
    reuseExistingServer: true,
    timeout: 10_000
  }
});
