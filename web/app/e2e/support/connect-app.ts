import { createHash, randomBytes } from "node:crypto";

import { request, type Page } from "@playwright/test";

import { ada } from "./accounts.ts";
import { openSession } from "./api.ts";
import { type Instance } from "./server.ts";

/** The address an app comes back to: this computer, on a port the app picked (RFC 8252). */
export const COMES_BACK_TO = "http://127.0.0.1:51734/callback";
/** The address an app registers: the port is not part of it. */
const REGISTERED = "http://127.0.0.1/callback";

/** The web address of an app that does not run on this computer, once an administrator allowed it. */
export const WEB_RETURN = "https://app.example.com/callback";

/** An administrator allows the web address of an app (Organization settings). */
export async function allowWebAddress(server: Instance): Promise<void> {
  const admin = openSession(server.url);
  await admin.client.POST("/api/v1/auth/sign-in", {
    body: { email: ada.email, password: ada.password },
  });
  const allowed = await admin.client.POST("/api/v1/oauth/web-addresses", {
    body: { address: WEB_RETURN },
  });
  if (!allowed.response.ok) {
    throw new Error(`the web address was not allowed: ${String(allowed.response.status)}`);
  }
  await admin.dispose();
}

/** An AI app that registered itself and is about to send a person to the consent step. */
export interface AppAtTheDoor {
  readonly name: string;
  readonly client: string;
  /** The address of the consent step, as the app sends the person to it. */
  readonly address: string;
  /** Swaps the code the person's *Allow* gave for tokens; the code is read from the address the page was sent to. */
  readonly exchange: (
    code: string,
  ) => Promise<{ readonly access: string; readonly refresh: string }>;
}

/** AI is on for the organization: a model on this computer needs no agreement. */
export async function switchAiOn(server: Instance): Promise<void> {
  const admin = openSession(server.url);
  await admin.client.POST("/api/v1/auth/sign-in", {
    body: { email: ada.email, password: ada.password },
  });
  const changed = await admin.client.PUT("/api/v1/ai", {
    body: {
      provider: "openai-compatible",
      address: "http://127.0.0.1/v1",
      model: "test-model",
      enabled: true,
      password: ada.password,
    },
  });
  if (!changed.response.ok) {
    throw new Error(`AI could not be switched on: ${String(changed.response.status)}`);
  }
  await admin.dispose();
}

/** A text member of an answer the server sent. */
function textOf(body: unknown, name: string): string {
  const value: unknown =
    typeof body === "object" && body !== null ? Reflect.get(body, name) : undefined;
  if (typeof value !== "string") {
    throw new TypeError(`the answer has no ${name}`);
  }
  return value;
}

/** A new proof for PKCE: the verifier an app keeps, and the challenge it sends. */
function proof(): { readonly verifier: string; readonly challenge: string } {
  const verifier = randomBytes(32).toString("base64url");
  return {
    verifier,
    challenge: createHash("sha256").update(verifier).digest("base64url"),
  };
}

/** An app registers itself and has the address of its consent step ready. */
export async function appAtTheDoor(
  server: Instance,
  name: string,
  scope: "see-only" | "see-and-change",
  onTheWeb = false,
): Promise<AppAtTheDoor> {
  const registeredAt = onTheWeb ? WEB_RETURN : REGISTERED;
  const comesBackTo = onTheWeb ? WEB_RETURN : COMES_BACK_TO;
  const web = await request.newContext({
    baseURL: server.url,
    ignoreHTTPSErrors: true,
  });
  const registered = await web.post("/oauth/register", {
    data: { client_name: name, redirect_uris: [registeredAt] },
  });
  const client = textOf(await registered.json(), "client_id");
  const { verifier, challenge } = proof();
  const query = new URLSearchParams({
    client_id: client,
    response_type: "code",
    redirect_uri: comesBackTo,
    code_challenge: challenge,
    code_challenge_method: "S256",
    scope,
    state: "state-1",
  });
  return {
    name,
    client,
    address: `/connect?${query.toString()}`,
    exchange: async (code) => {
      const tokens = await web.post("/oauth/token", {
        form: {
          grant_type: "authorization_code",
          client_id: client,
          code,
          redirect_uri: comesBackTo,
          code_verifier: verifier,
        },
      });
      const body: unknown = await tokens.json();
      return {
        access: textOf(body, "access_token"),
        refresh: textOf(body, "refresh_token"),
      };
    },
  };
}

/**
 * Catches the answer the browser is sent back to the app with. The address is on this computer and
 * nothing listens there, so the request is answered by the test; what it was sent to is returned.
 */
export async function catchTheAnswer(page: Page, press: () => Promise<void>): Promise<URL> {
  let answer: URL | undefined;
  await page.route("http://127.0.0.1:51734/**", async (route) => {
    answer = new URL(route.request().url());
    await route.fulfill({
      status: 200,
      contentType: "text/html",
      body: "<p>The app is back.</p>",
    });
  });
  await press();
  await page.waitForURL(/127\.0\.0\.1:51734/u);
  if (answer === undefined) {
    throw new Error("the browser was not sent back to the app");
  }
  return answer;
}

/** The code the answer to the app holds. */
export function codeOf(answer: string): string {
  const code = new URL(answer).searchParams.get("code");
  if (code === null) {
    throw new TypeError("the answer holds no code");
  }
  return code;
}
