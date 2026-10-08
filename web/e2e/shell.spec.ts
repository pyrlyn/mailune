import { expect, test } from "@playwright/test";

test("selects a fixture thread and shows its subject", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("button", { name: "Thursday", exact: true }).click();
  await expect(page.getByRole("region", { name: "Reading" })).toContainText("Thursday");
});
