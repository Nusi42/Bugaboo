import { dataOf } from "@bugaboo/api-client";
import { type Page } from "@playwright/test";

import { sessionOver } from "./api.ts";

/** What the server says about the two-step switch of the role whose page is open (the page may not have caught up). */
export async function twoStepOf(page: Page): Promise<unknown> {
  const { origin, pathname } = new URL(page.url());
  const role = pathname.split("/").at(-1) ?? "";
  const session = await sessionOver(page.context().request, origin);
  const answer = dataOf(
    await session.client.GET("/api/v1/roles/{role_id}", { params: { path: { role_id: role } } }),
  );
  return answer.role.two_step;
}
