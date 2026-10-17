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

/**
 * One conversation with the server, the few commands that sending one e-mail takes: the function
 * takes what came in and answers through `say`; it returns `true` once the server said goodbye.
 */
function conversation(
  keep: (mail: Mail) => void,
  say: (line: string) => void,
): (text: string) => boolean {
  let to = "";
  let data: string | undefined;
  let pending = "";
  const finishData = (): boolean => {
    const end = pending.indexOf(TEXT_END);
    if (end >= 0) {
      keep({ to, text: decoded((data ?? "") + pending.slice(0, end)) });
      pending = pending.slice(end + TEXT_END.length);
      data = undefined;
      say("250 queued");
    }
    return end >= 0;
  };
  const answer = (line: string): boolean => {
    const command = line.toUpperCase();
    if (command === "QUIT") {
      say("221 bye");
      return true;
    }
    to = command.startsWith("RCPT TO") ? (/<([^>]*)>/u.exec(line)?.[1] ?? "") : to;
    data = command === "DATA" ? "" : data;
    say(command === "DATA" ? "354 go on" : "250 ok");
    return false;
  };
  say("220 test ESMTP");
  return (text) => {
    pending += text;
    for (;;) {
      if (data !== undefined) {
        if (!finishData()) {
          return false;
        }
        continue;
      }
      const lineEnd = pending.indexOf(LINE_END);
      if (lineEnd < 0) {
        return false;
      }
      const line = pending.slice(0, lineEnd);
      pending = pending.slice(lineEnd + LINE_END.length);
      if (answer(line)) {
        return true;
      }
    }
  };
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
 * Starts a mail server on a free port of this computer. It stays up until the test process ends
 * (the server of a file may be used again by a repeat of its tests).
 */
export async function startMailbox(): Promise<Mailbox> {
  const took: Mail[] = [];
  const { port, connections } = await listening();
  void (async () => {
    for await (const [arrived] of connections) {
      const socket: unknown = arrived;
      if (!(socket instanceof Socket)) {
        continue;
      }
      const feed = conversation(
        (mail) => {
          took.push(mail);
        },
        (line) => {
          socket.write(`${line}${LINE_END}`);
        },
      );
      void (async () => {
        try {
          for await (const chunk of socket) {
            if (feed(String(chunk))) {
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
