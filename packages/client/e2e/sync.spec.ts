import { test, expect } from "@playwright/test";

test.describe("Sync Protocol E2E", () => {
  test("shows connected status on load", async ({ page }) => {
    await page.goto("/");
    await expect(page.getByText("Connected")).toBeVisible({ timeout: 10_000 });
  });

  test("initial load shows no orders", async ({ page }) => {
    await page.goto("/");
    await expect(page.getByText("No orders yet")).toBeVisible();
  });

  test("place order and see it appear (Full message)", async ({ page }) => {
    await page.goto("/");
    await expect(page.getByText("Connected")).toBeVisible({ timeout: 10_000 });

    await page.locator("input[type='text']").fill("ETH");
    await page.getByRole("button", { name: "Place Order" }).click();

    // Order card should appear with the symbol
    await expect(page.locator(".bg-card").filter({ hasText: "ETH" })).toBeVisible({ timeout: 5_000 });
    await expect(page.getByText("open")).toBeVisible();
  });

  test("update order changes only the patched field (Patch message)", async ({ page }) => {
    await page.goto("/");
    await expect(page.getByText("Connected")).toBeVisible({ timeout: 10_000 });

    // Place an order
    await page.locator("input[type='text']").fill("BTC");
    await page.getByRole("button", { name: "Place Order" }).click();
    await expect(page.locator(".bg-card").filter({ hasText: "BTC" })).toBeVisible({ timeout: 5_000 });

    // Update the first order
    await page.getByRole("button", { name: "Update First Order" }).click();

    // Status should change to "partially_filled"
    await expect(page.getByText("partially_filled")).toBeVisible({ timeout: 5_000 });
    // Filled quantity should show "500"
    await expect(page.getByText("500")).toBeVisible();
    // Symbol should still be present (unchanged by patch)
    await expect(page.locator(".bg-card").filter({ hasText: "BTC" })).toBeVisible();
  });

  test("multiple orders appear in grid", async ({ page }) => {
    await page.goto("/");
    await expect(page.getByText("Connected")).toBeVisible({ timeout: 10_000 });

    // Place two orders
    await page.locator("input[type='text']").fill("BTC");
    await page.getByRole("button", { name: "Place Order" }).click();
    await expect(page.locator(".bg-card").filter({ hasText: "BTC" })).toBeVisible({ timeout: 5_000 });

    await page.locator("input[type='text']").fill("ETH");
    await page.getByRole("button", { name: "Place Order" }).click();
    await expect(page.locator(".bg-card").filter({ hasText: "ETH" })).toBeVisible({ timeout: 5_000 });

    // Both order cards should be visible
    await expect(page.locator(".bg-card")).toHaveCount(2);
  });

  test("order persists in store after placement", async ({ page }) => {
    await page.goto("/");
    await expect(page.getByText("Connected")).toBeVisible({ timeout: 10_000 });

    // Place an order
    await page.locator("input[type='text']").fill("SOL");
    await page.getByRole("button", { name: "Place Order" }).click();
    await expect(page.locator(".bg-card").filter({ hasText: "SOL" })).toBeVisible({ timeout: 5_000 });

    // Verify the order card shows all expected fields
    const card = page.locator(".bg-card").filter({ hasText: "SOL" });
    await expect(card.getByText("buy")).toBeVisible();
    await expect(card.getByText("open")).toBeVisible();
    await expect(card.getByText("ID:")).toBeVisible();
  });
});
