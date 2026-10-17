import { on } from "node:events";
import { createServer, Socket } from "node:net";

import { settingKeys } from "@bugaboo/api-client";
import { expect } from "@playwright/test";

import { type ApiSession } from "./api.ts";
import { inOrder } from "./sequence.ts";

/** One e-mail the mail server of a test took. */
export interface Mail {
  /** The address it went to. */
  readonly to: string;
  /** The text, with the encoding of the lines undone. */
  readonly text: string;
}

/** The user name and the password a mail server of a test asks for; without them it asks for none. */
export interface Login {
  readonly user: string;
  readonly password: string;
}

/** A mail server for a test: it takes what the server sends and keeps it to be read. */
export interface Mailbox {
  /** The port on this computer that the server is told to send e-mail to. */
  readonly port: number;
  /** Everything it took so far, oldest first. */
  readonly mails: () => readonly Mail[];
  /** The first e-mail to `address`, waiting for it while the job that sends it comes. */
  readonly waitFor: (address: string) => Promise<Mail>;
}

/** How long a test waits for an e-mail: the job that sends it comes every few seconds. */
const WAIT_MS = 20_000;
/** The end of a line, and of the text of an e-mail, in the language of mail servers. */
const LINE_END = "\r\n";
const TEXT_END = `${LINE_END}.${LINE_END}`;

/** The text of an e-mail body without the soft line breaks and escapes of the quoted-printable form. */
function decoded(raw: string): string {
  return raw
    .replaceAll(/=\r?\n/gu, "")
    .replaceAll(/=([0-9A-F]{2})/gu, (_, hex: string) =>
      String.fromCodePoint(Number.parseInt(hex, 16)),
    );
}

/** What the mail server says to the commands of signing in, when it asks for a login. */
const AUTH_OK = "235 authenticated";
const AUTH_FAILED = "535 5.7.8 authentication failed";
const NOT_SIGNED_IN = "530 5.7.0 authentication required";

/** Whether the base64 text is the `PLAIN` answer for this login: a NUL character, the user, a NUL character, the password. */
function isLogin(encoded: string, login: Login): boolean {
  const NUL = String.fromCodePoint(0);
  return Buffer.from(encoded, "base64").toString() === `${NUL}${login.user}${NUL}${login.password}`;
}

/** What the server of a test says to a line. */
type Say = (line: string) => void;

/** What the dialogue about signing in did with a line. */
type SignIn = "not-about-it" | "offered" | "signed-in" | "refused";

/** The part of the dialogue about signing in: `EHLO` offers `AUTH PLAIN`, `AUTH PLAIN` signs in or not. */
function signInStep(line: string, login: Login, say: Say): SignIn {
  const command = line.toUpperCase();
  if (command.startsWith("EHLO")) {
    say("250-test");
    say("250 AUTH PLAIN");
    return "offered";
  }
  if (!command.startsWith("AUTH PLAIN")) {
    return "not-about-it";
  }
  const signedIn = isLogin(line.slice("AUTH PLAIN".length).trim(), login);
  say(signedIn ? AUTH_OK : AUTH_FAILED);
  return signedIn ? "signed-in" : "refused";
}

/**
 * One conversation with the server, the few commands that sending one e-mail takes. It is fed what
 * came in and answers through `say`; `feed` returns `true` once the server said goodbye. With a
 * `login` the server offers `AUTH PLAIN` and takes e-mail only from whoever signed in.
 */
class Conversation {
  private to = "";
  private data: string | undefined;
  private pending = "";
  private signedIn: boolean;
  private readonly keep: (mail: Mail) => void;
  private readonly say: Say;
  private readonly login: Login | undefined;

  constructor(keep: (mail: Mail) => void, say: Say, login?: Login) {
    this.keep = keep;
    this.say = say;
    this.login = login;
    this.signedIn = login === undefined;
    say("220 test ESMTP");
  }

  /** Takes in what came and answers every line of it. */
  feed(text: string): boolean {
    this.pending += text;
    for (;;) {
      if (this.data !== undefined) {
        if (!this.finishData()) {
          return false;
        }
        continue;
      }
      const lineEnd = this.pending.indexOf(LINE_END);
      if (lineEnd < 0) {
        return false;
      }
      const line = this.pending.slice(0, lineEnd);
      this.pending = this.pending.slice(lineEnd + LINE_END.length);
      if (this.answer(line)) {
        return true;
      }
    }
  }

  /** The text of the e-mail is complete when its end is there: keeps it. */
  private finishData(): boolean {
    const end = this.pending.indexOf(TEXT_END);
    if (end >= 0) {
      this.keep({ to: this.to, text: decoded((this.data ?? "") + this.pending.slice(0, end)) });
      this.pending = this.pending.slice(end + TEXT_END.length);
      this.data = undefined;
      this.say("250 queued");
    }
    return end >= 0;
  }

  /** Answers one command; `true` after `QUIT`. */
  private answer(line: string): boolean {
    const command = line.toUpperCase();
    if (command === "QUIT") {
      this.say("221 bye");
      return true;
    }
    const step = this.login === undefined ? "not-about-it" : signInStep(line, this.login, this.say);
    if (step !== "not-about-it") {
      this.signedIn = step === "signed-in";
    } else if (!this.signedIn && command.startsWith("MAIL")) {
      this.say(NOT_SIGNED_IN);
    } else {
      this.to = command.startsWith("RCPT TO") ? (/<([^>]*)>/u.exec(line)?.[1] ?? "") : this.to;
      this.data = command === "DATA" ? "" : this.data;
      this.say(command === "DATA" ? "354 go on" : "250 ok");
    }
    return false;
  }
}

/** A server that listens on a free port of this computer, and the connections it gets. */
async function listening(): Promise<{
  readonly port: number;
  readonly connections: AsyncIterable<readonly unknown[]>;
}> {
  const listener = createServer();
  listener.unref();
  const port = await new Promise<number>((done, failed) => {
    listener.once("error", failed);
    listener.listen(0, "127.0.0.1", () => {
      const address = listener.address();
      if (address === null || typeof address === "string") {
        failed(new Error("the mail server has no port"));
      } else {
        done(address.port);
      }
    });
  });
  return { port, connections: on(listener, "connection") };
}

/**
 * Starts a mail server on a free port of this computer, one that asks for `login` if one is given.
 * It stays up until the test process ends (the server of a file may be used again by a repeat of
 * its tests).
 */
export async function startMailbox(login?: Login): Promise<Mailbox> {
  const took: Mail[] = [];
  const { port, connections } = await listening();
  void (async () => {
    for await (const [arrived] of connections) {
      const socket: unknown = arrived;
      if (!(socket instanceof Socket)) {
        continue;
      }
      const conversation = new Conversation(
        (mail) => {
          took.push(mail);
        },
        (line) => {
          socket.write(`${line}${LINE_END}`);
        },
        login,
      );
      void (async () => {
        try {
          for await (const chunk of socket) {
            if (conversation.feed(String(chunk))) {
              socket.end();
            }
          }
        } catch {
          socket.destroy();
        }
      })();
    }
  })();
  const mailTo = (address: string): Mail | undefined => took.find((mail) => mail.to === address);
  return {
    port,
    mails: () => took,
    waitFor: async (address) => {
      await expect
        .poll(() => mailTo(address), {
          timeout: WAIT_MS,
          message: `no e-mail came for ${address}`,
        })
        .toBeDefined();
      const found = mailTo(address);
      if (found === undefined) {
        throw new Error(`no e-mail came for ${address}`);
      }
      return found;
    },
  };
}

/** Enters the mail server of `mailbox` as the mail server of the organization and switches e-mail on. */
export async function turnMailOn(admin: ApiSession, mailbox: Mailbox): Promise<void> {
  const settings: readonly (readonly [string, string | number | boolean])[] = [
    [settingKeys.mailHost, "127.0.0.1"],
    [settingKeys.mailPort, mailbox.port],
    // Nothing to protect on this computer: no user name, no password, so no encryption is allowed.
    [settingKeys.mailSecurity, "off"],
    [settingKeys.mailFrom, "bugaboo@example.test"],
    [settingKeys.mailEnabled, true],
  ];
  await inOrder(
    settings.map(([key, value]) => async () => {
      await admin.client.PUT("/api/v1/settings/{key}", {
        params: { path: { key } },
        body: { value },
      });
    }),
  );
}

/** The address of the invitation page in an e-mail: the path and the secret after the hash sign. */
export function invitationPath(mail: Mail): string {
  const found = /https?:\/\/\S+\/invitation#\S+/u.exec(mail.text);
  if (found === null) {
    throw new Error(`no link to the invitation page in the e-mail: ${mail.text}`);
  }
  const link = new URL(found[0]);
  return `${link.pathname}${link.hash}`;
}
