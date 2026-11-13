import { type Page } from "@playwright/test";

/**
 * Holds back the answers about the layout of the new-issue form, as a slow server would, until the returned function is
 * called.
 */
export async function holdNewIssueLayout(page: Page): Promise<() => void> {
  const held = Promise.withResolvers<void>();
  await page.route("**/api/v1/layouts/form?*kind=new-issue*", async (route) => {
    await held.promise;
    await route.continue();
  });
  return held.resolve;
}
