import { dataOf, type Schema } from "@bugaboo/api-client";

import { type ApiSession, openSession } from "./api.ts";
import { type CodeApp, codeApp } from "./totp.ts";
import { type Instance } from "./server.ts";
import { inOrder } from "./sequence.ts";

/** Somebody who has, or is going to have, an account. */
export interface Person {
  readonly name: string;
  readonly email: string;
  /** A password that is long enough and on no list of breached passwords. */
  readonly password: string;
}

export const ada: Person = {
  name: "Ada Admin",
  email: "ada@example.test",
  password: "Tr4mpoline-Quiet-Harbor-81",
};

export const bob: Person = {
  name: "Bob Builder",
  email: "bob@example.test",
  password: "Violet-Lantern-Marsh-5520", // gitleaks:allow -- fake test credential, SEC-16
};

export const cleo: Person = {
  name: "Cleo Coder",
  email: "cleo@example.test",
  password: "Granite-Orbit-Meadow-3047",
};

/** Makes the first administrator through the API, with the setup code the server saved. */
export async function createAdmin(server: Instance, person: Person = ada): Promise<ApiSession> {
  const session = openSession(server.url);
  await session.client.POST("/api/v1/auth/setup", {
    body: {
      code: server.setupCode(),
      name: person.name,
      email: person.email,
      password: person.password,
    },
  });
  return session;
}

/** Signs the person in with their password, through the API (a person who has no second step). */
export async function signInThroughApi(server: Instance, person: Person): Promise<ApiSession> {
  const session = openSession(server.url);
  await session.client.POST("/api/v1/auth/sign-in", {
    body: { email: person.email, password: person.password },
  });
  return session;
}

/** The levels at which a person's or an organization's settings are kept, by the routes that serve them. */
const settingsRoutes = {
  person: { list: "/api/v1/me/settings", one: "/api/v1/me/settings/{key}" },
  organization: { list: "/api/v1/settings", one: "/api/v1/settings/{key}" },
} as const;

/**
 * Takes away every value the person chose in *My settings* (and, when asked, for an administrator,
 * every value of the organization with its lock), so a test starts from the standard settings: what a person
 * chose is kept on the server now, and the tests of one file share a server.
 */
export async function forgetChoices(
  server: Instance,
  person: Person = ada,
  organization = false,
): Promise<void> {
  const session = await signInThroughApi(server, person);
  await inOrder(
    (organization
      ? [settingsRoutes.person, settingsRoutes.organization]
      : [settingsRoutes.person]
    ).map((level) => async () => {
      const found: Schema<"SettingsBody"> = dataOf(await session.client.GET(level.list));
      await inOrder(
        found.settings
          .filter((setting) => setting.own)
          .map((setting) => async () => {
            await session.client.DELETE(level.one, { params: { path: { key: setting.key } } });
          }),
      );
    }),
  );
  await session.dispose();
}

/** The secret in an invitation link: the part after the hash sign. */
function secretOf(link: string): string {
  return link.slice(link.indexOf("#") + 1);
}

/** Makes an invitation as `admin` and returns its link path (the page and the secret). */
export async function invite(admin: ApiSession, email: string): Promise<string> {
  const made = dataOf(await admin.client.POST("/api/v1/auth/invitations", { body: { email } }));
  return made.link_path;
}

/** Makes an account for `person` with an invitation from `admin`, and signs the person in. */
export async function createPerson(
  server: Instance,
  admin: ApiSession,
  person: Person,
): Promise<ApiSession> {
  const link = await invite(admin, person.email);
  const session = openSession(server.url);
  await session.client.POST("/api/v1/auth/invitations/accept", {
    body: { secret: secretOf(link), name: person.name, password: person.password },
  });
  return session;
}

/** Signs the person out of every browser, so a test starts with no place to be listed. */
export async function endEverySession(server: Instance, person: Person): Promise<void> {
  const session = await signInThroughApi(server, person);
  await session.client.DELETE("/api/v1/auth/sessions");
}

/** A code app and the backup codes that came with setting it up. */
export interface TwoStep {
  readonly app: CodeApp;
  readonly backupCodes: readonly string[];
}

/** Sets up the code app for the person behind `session`, as the security page would. */
export async function addCodeApp(session: ApiSession, password: string): Promise<TwoStep> {
  const started = dataOf(
    await session.client.POST("/api/v1/auth/two-step/code-app", { body: { password } }),
  );
  const app = codeApp(started.secret);
  const confirmed = dataOf(
    await session.client.POST("/api/v1/auth/two-step/code-app/confirm", {
      body: { code: await app.next(), password },
    }),
  );
  return { app, backupCodes: confirmed.codes };
}

/** The ID of the person behind a session. */
export async function ownId(session: ApiSession): Promise<string> {
  return dataOf(await session.client.GET("/api/v1/auth/me")).id;
}

/** Sets who may sign up on their own, through the API. */
export async function setSignUpPolicy(
  admin: ApiSession,
  policy: { readonly enabled: boolean; readonly domains: readonly string[] },
): Promise<void> {
  // A change is made on the version that was read.
  const { version } = dataOf(await admin.client.GET("/api/v1/auth/sign-up-policy"));
  await admin.client.PUT("/api/v1/auth/sign-up-policy", {
    body: { enabled: policy.enabled, domains: [...policy.domains], version },
  });
}
