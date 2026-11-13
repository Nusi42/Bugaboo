import { dataOf } from "@bugaboo/api-client";
import { type Page } from "@playwright/test";

import { signInThroughApi } from "./accounts.ts";
import { type ApiSession } from "./api.ts";
import { readIssue } from "./issues.ts";
import { sampleAdministrator } from "./sample.ts";
import { type Instance } from "./server.ts";

/** The sample's administrator, through the API: who the swipe tests check and put back. */
export function adminSession(server: Instance): Promise<ApiSession> {
  return signInThroughApi(server, sampleAdministrator());
}

/** Whether the person of the session follows the issue, as the server says. */
export async function followsIssue(session: ApiSession, key: string): Promise<boolean> {
  return dataOf(
    await session.client.GET("/api/v1/issues/{issue}/follow", {
      params: { path: { issue: key } },
    }),
  ).following;
}

/** Puts back whether the person of the session follows the issue. */
export async function setFollowing(
  session: ApiSession,
  key: string,
  following: boolean,
): Promise<void> {
  const params = { params: { path: { issue: key } } };
  await (following
    ? session.client.PUT("/api/v1/issues/{issue}/follow", params)
    : session.client.DELETE("/api/v1/issues/{issue}/follow", params));
}

/** The status of an issue as the server says: its key and its name. */
export async function statusNow(
  session: ApiSession,
  key: string,
): Promise<{ readonly key: string; readonly name: string; readonly version: number }> {
  const { issue } = await readIssue(session, { key });
  return {
    key: issue.status?.key ?? "",
    name: issue.status?.name ?? "",
    version: issue.version ?? 0,
  };
}

/** Puts the status of an issue back, on the version it has now. */
export async function putStatusBack(
  session: ApiSession,
  key: string,
  status: string,
): Promise<void> {
  const { version } = await statusNow(session, key);
  dataOf(
    await session.client.PATCH("/api/v1/issues/{issue}", {
      params: { path: { issue: key } },
      body: { version, status },
    }),
  );
}

/** Makes the server refuse the next *Stop following* of the page, as it refuses a person who may not do it. */
export async function refuseStopFollowing(page: Page): Promise<void> {
  const code = "access.forbidden";
  await page.route("**/api/v1/issues/*/follow", (route) =>
    route.request().method() === "DELETE"
      ? route.fulfill({
          status: 403,
          contentType: "application/problem+json",
          body: JSON.stringify({
            type: `urn:bugaboo:error:${code}`,
            title: "Refused",
            status: 403,
            code,
            message_key: `error.${code}`,
            params: {},
          }),
        })
      : route.fallback(),
  );
}
