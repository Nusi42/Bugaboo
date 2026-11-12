import { settingKeys } from "@bugaboo/api-client";
import { type Page } from "@playwright/test";

import { type Person, signInThroughApi } from "./accounts.ts";
import { type Instance } from "./server.ts";

/** An own theme as a test needs it: its ID and the address of its style sheet. */
export interface OwnTheme {
  readonly id: string;
  readonly sheet: string;
}

/** The name the page keeps the refusals of its content security policy under. */
const REFUSED = "refusedByPolicy";

/**
 * Makes `person` a personal theme that extends Dusk with these colors (an upload, which is saved
 * at once) and chooses it as their dark theme, always dark.
 */
export async function chooseOwnDarkTheme(
  server: Instance,
  person: Person,
  colors: Readonly<Record<string, { readonly value: string }>>,
): Promise<OwnTheme> {
  const session = await signInThroughApi(server, person);
  const uploaded = await session.client.POST("/api/v1/themes/upload", {
    params: { query: { kind: "personal" } },
    body: { name: "Evening", extends: "dusk", values: { colors } },
  });
  const theme = uploaded.data?.theme;
  const sheet = theme?.sheet;
  if (theme === undefined || typeof sheet !== "string") {
    throw new Error("the theme was not saved");
  }
  await session.client.PUT("/api/v1/me/settings/{key}", {
    params: { path: { key: settingKeys.appearanceMode } },
    body: { value: "dark" },
  });
  const chosen = await session.client.PUT("/api/v1/me/settings/{key}", {
    params: { path: { key: settingKeys.appearanceDarkTheme } },
    body: { value: theme.choice },
  });
  await session.dispose();
  if (!chosen.response.ok) {
    throw new Error("the theme could not be chosen");
  }
  return { id: theme.id, sheet };
}

/** Makes the page write down every refusal of its content security policy, from its first script on. */
export async function watchThePolicy(page: Page): Promise<void> {
  await page.addInitScript((name: string) => {
    const refused: string[] = [];
    Reflect.set(window, name, refused);
    document.addEventListener("securitypolicyviolation", () => {
      refused.push(document.location.pathname);
    });
  }, REFUSED);
}

/** What the content security policy of the page refused so far. */
export function refusedByThePolicy(page: Page): Promise<unknown> {
  return page.evaluate((name: string) => Reflect.get(window, name) as unknown, REFUSED);
}

/** The value of a custom property on the frame of the page, as the browser works it out. */
export function frameVariable(page: Page, name: string): Promise<string> {
  return page.evaluate(
    (variable: string) =>
      getComputedStyle(document.documentElement).getPropertyValue(variable).trim(),
    name,
  );
}
