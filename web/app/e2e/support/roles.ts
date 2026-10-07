import { type Page } from "@playwright/test";

/** What the server says about the two-step switch of the role whose page is open (the page may not have caught up). */
export async function twoStepOf(page: Page): Promise<unknown> {
  const role = new URL(page.url()).pathname.split("/").at(-1) ?? "";
  const answer = await page.context().request.get(`/api/v1/roles/${role}`);
  const body: unknown = await answer.json();
  const detail: unknown =
    typeof body === "object" && body !== null ? Reflect.get(body, "role") : undefined;
  return typeof detail === "object" && detail !== null
    ? Reflect.get(detail, "two_step")
    : undefined;
}
