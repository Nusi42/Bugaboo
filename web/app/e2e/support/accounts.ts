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
export async function makeAdmin(server: Instance, person: Person = ada): Promise<ApiSession> {
  const session = await openSession(server.url);
  await session.post("/api/v1/auth/setup", {
    code: server.setupCode(),
    display_name: person.name,
    email: person.email,
    password: person.password,
  });
  return session;
}

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
  const session = await openSession(server.url);
  await session.post("/api/v1/auth/sign-in", { email: person.email, password: person.password });
  await inOrder(
    (organization ? ["me/settings", "settings"] : ["me/settings"]).map((level) => async () => {
      const found = await session.get(`/api/v1/${level}`);
      const settings: readonly unknown[] = Array.isArray(found["settings"])
        ? found["settings"]
        : [];
      await inOrder(
        settings
          .flatMap((setting) => ownKey(setting))
          .map((key) => async () => {
            await session.remove(`/api/v1/${level}/${key}`);
          }),
      );
    }),
  );
  await session.dispose();
}

/** The key of a setting that has a value of its own at this level, as a list of one, or nothing. */
function ownKey(setting: unknown): readonly string[] {
  if (typeof setting !== "object" || setting === null) {
    return [];
  }
  const own: unknown = Reflect.get(setting, "own");
  const key: unknown = Reflect.get(setting, "key");
  return own === true && typeof key === "string" ? [key] : [];
}

/** The secret in an invitation link: the part after the hash sign. */
function secretOf(link: string): string {
  return link.slice(link.indexOf("#") + 1);
}

/** Makes an invitation as `admin` and returns its link path (the page and the secret). */
export async function invite(admin: ApiSession, email: string): Promise<string> {
  const made = await admin.post("/api/v1/auth/invitations", { email });
  return String(made["link_path"]);
}

/** Makes an account for `person` with an invitation from `admin`, and signs the person in. */
export async function makePerson(
  server: Instance,
  admin: ApiSession,
  person: Person,
): Promise<ApiSession> {
  const link = await invite(admin, person.email);
  const session = await openSession(server.url);
  await session.post("/api/v1/auth/invitation/accept", {
    secret: secretOf(link),
    display_name: person.name,
    password: person.password,
  });
  return session;
}

/** Signs the person out of every browser, so a test starts with no place to be listed. */
export async function endEverySession(server: Instance, person: Person): Promise<void> {
  const session = await openSession(server.url);
  await session.post("/api/v1/auth/sign-in", { email: person.email, password: person.password });
  await session.remove("/api/v1/auth/sessions");
}

/** A code app and the backup codes that came with setting it up. */
export interface TwoStep {
  readonly app: CodeApp;
  readonly backupCodes: readonly string[];
}

/** Sets up the code app for the person behind `session`, as the security page would. */
export async function addCodeApp(session: ApiSession, password: string): Promise<TwoStep> {
  const started = await session.post("/api/v1/auth/two-step/code-app", { password });
  const app = codeApp(String(started["secret"]));
  const confirmed = await session.post("/api/v1/auth/two-step/code-app/confirm", {
    code: await app.next(),
    password,
  });
  const codes = confirmed["codes"];
  return { app, backupCodes: Array.isArray(codes) ? codes.map(String) : [] };
}
