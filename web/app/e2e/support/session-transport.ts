import { Agent as PlainAgent, request as sendPlain } from "node:http";
import { Agent as SecureAgent, request as sendSecure } from "node:https";

/**
 * What the API sessions of the tests talk through (M4 WP-03): one pool of connections for each
 * server, kept alive and shared by every session of the file, instead of the connections of a
 * Playwright request context for each session that were all closed again when it ended. A closed
 * connection stays in TIME_WAIT for two minutes, and the sessions of a file are many. Each session
 * keeps its own cookies in a [`CookieJar`], so two people never share a sign-in.
 */

/** The time a request gets before the test gives up on it. */
const REQUEST_LIMIT_MS = 30_000;

/** One cookie the way Playwright's storage state holds it, to open a browser as the session. */
export interface StoredCookie {
  readonly name: string;
  readonly value: string;
  readonly domain: string;
  readonly path: string;
  /** Seconds since 1970, or -1 for a cookie that ends with the session. */
  readonly expires: number;
  readonly httpOnly: boolean;
  readonly secure: boolean;
  readonly sameSite: "Strict" | "Lax" | "None";
}

/** A cookie that ends with the session, as the `Set-Cookie` line starts it. */
function startedCookie(
  line: string,
  domain: string,
): { -readonly [Key in keyof StoredCookie]: StoredCookie[Key] } {
  const [pair = ""] = line.split(";");
  const at = pair.indexOf("=");
  return {
    name: pair.slice(0, at).trim(),
    value: pair.slice(at + 1).trim(),
    domain,
    path: "/",
    expires: -1,
    httpOnly: false,
    secure: false,
    sameSite: "Lax",
  };
}

/** The `SameSite` words of a `Set-Cookie` line, as the storage state spells them. */
function sameSiteOf(value: string, before: StoredCookie["sameSite"]): StoredCookie["sameSite"] {
  switch (value.toLowerCase()) {
    case "strict":
      return "Strict";
    case "none":
      return "None";
    default:
      return before;
  }
}

/** The cookies of one session. */
export class CookieJar {
  private readonly cookies = new Map<string, StoredCookie>();

  private readonly host: string;

  public constructor(host: string) {
    this.host = host;
  }

  /** Takes in the `Set-Cookie` lines of an answer: a cookie that has ended is forgotten. */
  public store(lines: readonly string[]): void {
    for (const line of lines) {
      const cookie = this.parse(line);
      const key = `${cookie.name} ${cookie.path}`;
      if (cookie.expires !== -1 && cookie.expires * 1000 <= Date.now()) {
        this.cookies.delete(key);
      } else {
        this.cookies.set(key, cookie);
      }
    }
  }

  /** The `Cookie` header for a request to this path, or nothing when there is no cookie for it. */
  public header(path: string): string | undefined {
    const matching = this.all().filter((cookie) => path.startsWith(cookie.path));
    return matching.length === 0
      ? undefined
      : matching.map((cookie) => `${cookie.name}=${cookie.value}`).join("; ");
  }

  /** The value of a cookie by its name. */
  public valueOf(name: string): string | undefined {
    return this.all().find((cookie) => cookie.name === name)?.value;
  }

  /** Every cookie that has not ended, as a browser context takes them. */
  public all(): StoredCookie[] {
    return [...this.cookies.values()].filter(
      (cookie) => cookie.expires === -1 || cookie.expires * 1000 > Date.now(),
    );
  }

  private parse(line: string): StoredCookie {
    const cookie = startedCookie(line, this.host);
    for (const attribute of line.split(";").slice(1)) {
      const [name = "", value = ""] = attribute.trim().split("=");
      switch (name.toLowerCase()) {
        case "path":
          cookie.path = value;
          break;
        case "max-age":
          cookie.expires = Math.floor(Date.now() / 1000) + Number(value);
          break;
        case "expires":
          cookie.expires = cookie.expires === -1 ? Date.parse(value) / 1000 : cookie.expires;
          break;
        case "httponly":
          cookie.httpOnly = true;
          break;
        case "secure":
          cookie.secure = true;
          break;
        case "samesite":
          cookie.sameSite = sameSiteOf(value, cookie.sameSite);
          break;
        default:
          break;
      }
    }
    return cookie;
  }
}

/** What a request asks for. */
export interface Asked {
  readonly url: string;
  readonly method: string;
  readonly headers: Readonly<Record<string, string>>;
  readonly body: string;
}

/** What the server answered. */
export interface Answered {
  readonly status: number;
  /** The headers as the server sent them, one by one (a few cookies are a few `set-cookie`). */
  readonly headers: readonly (readonly [string, string])[];
  readonly body: string;
}

const pools = new Map<string, PlainAgent | SecureAgent>();

/** The pool of connections for the server at `origin`; made on the first use. */
function poolFor(origin: string): PlainAgent | SecureAgent {
  const found = pools.get(origin);
  if (found !== undefined) {
    return found;
  }
  // Plain HTTP is for the unit test of the pool; the servers of the tests speak HTTPS.
  const made = origin.startsWith("http:")
    ? new PlainAgent({ keepAlive: true })
    : new SecureAgent({ keepAlive: true });
  pools.set(origin, made);
  return made;
}

/** Closes the connections to a server that is stopping; the next use makes a new pool. */
export function closePool(origin: string): void {
  pools.get(origin)?.destroy();
  pools.delete(origin);
}

/** Pairs of a flat list of headers (name, value, name, value, …) as Node hands them out. */
function pairsOf(raw: readonly string[]): [string, string][] {
  const pairs: [string, string][] = [];
  for (let at = 0; at + 1 < raw.length; at += 2) {
    pairs.push([raw[at] ?? "", raw[at + 1] ?? ""]);
  }
  return pairs;
}

/** One request over the pool. */
function once(asked: Asked, origin: string): Promise<{ answered: Answered; reused: boolean }> {
  return new Promise((done, failed) => {
    const send = asked.url.startsWith("http:") ? sendPlain : sendSecure;
    const out = send(
      asked.url,
      {
        method: asked.method,
        headers: asked.headers,
        agent: poolFor(origin),
        // The servers use the certificate they made themselves.
        rejectUnauthorized: false,
      },
      (answer) => {
        let body = "";
        answer.setEncoding("utf8");
        answer.on("data", (chunk: string) => {
          body += chunk;
        });
        answer.once("end", () => {
          done({
            answered: {
              status: answer.statusCode ?? 0,
              headers: pairsOf(answer.rawHeaders),
              body,
            },
            reused: out.reusedSocket,
          });
        });
        answer.once("error", failed);
      },
    );
    out.setTimeout(REQUEST_LIMIT_MS, () => {
      out.destroy(new Error(`No answer from ${asked.url} in ${String(REQUEST_LIMIT_MS)} ms.`));
    });
    out.once("error", (error: unknown) => {
      const why = error instanceof Error ? error.message : String(error);
      failed(
        Object.assign(new Error(`The request to ${asked.url} failed: ${why}`, { cause: error }), {
          reused: out.reusedSocket,
          code: error instanceof Error && "code" in error ? error.code : undefined,
        }),
      );
    });
    out.end(asked.body === "" ? undefined : asked.body);
  });
}

/** Whether the server had closed a connection that was kept, before the request reached it. */
function wasClosedWhileIdle(error: unknown): boolean {
  return (
    error instanceof Error &&
    "reused" in error &&
    error.reused === true &&
    "code" in error &&
    (error.code === "ECONNRESET" || error.code === "EPIPE")
  );
}

/**
 * Sends a request through the pool of the server at `origin`. A connection that the server closed
 * while it was idle (the pool cannot know in time) is replaced once; any other failure is a failure.
 */
export async function exchange(asked: Asked, origin: string): Promise<Answered> {
  try {
    return (await once(asked, origin)).answered;
  } catch (error) {
    if (wasClosedWhileIdle(error)) {
      return (await once(asked, origin)).answered;
    }
    throw error;
  }
}
