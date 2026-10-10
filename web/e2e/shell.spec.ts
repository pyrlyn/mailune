import { expect, test } from "@playwright/test";

// Subjects come from web/src/fixture.ts; UI text is matched by role, not wording.

test("choosing a fixture thread shows its subject", async ({ page }) => {
  await page.goto("/");
  const reading = page.getByRole("main");
  await page.getByRole("button", { name: /Thursday/ }).click();
  await expect(reading.getByRole("heading", { level: 2 })).toHaveText("Thursday");
  await expect(reading).toContainText("grace@example.com");
});

test("the shell still opens offline once the service worker has cached it", async ({ page, context }) => {
  await page.goto("/");
  await page.evaluate(async () => {
    await navigator.serviceWorker.ready;
  });
  // The first load ran before the worker existed; this one goes through it and fills the cache.
  await page.reload();
  await expect.poll(() => page.evaluate(() => navigator.serviceWorker.controller !== null)).toBe(true);

  await context.setOffline(true);
  await page.reload();
  await page.getByRole("button", { name: /Thursday/ }).click();
  await expect(page.getByRole("main").getByRole("heading", { level: 2 })).toHaveText("Thursday");
});
