import { createServer, type IncomingMessage, type ServerResponse } from "node:http";

/** What the fake model says next: some text, or a call of a tool of the assistant. */
export type Said =
  | { readonly text: string }
  | { readonly tool: string; readonly arguments: Readonly<Record<string, unknown>> };

/** A model on this computer that speaks the chat shape of `OpenAI` and says what a test scripts. */
export interface FakeModel {
  /** The address to enter as the address of the model. */
  readonly address: string;
  /** What the model says to the next calls, one for each call. */
  readonly will: (...said: readonly Said[]) => void;
  /** The text of every turn of the person that the model was given, one list for each call. */
  readonly given: () => readonly (readonly string[])[];
  readonly close: () => Promise<void>;
}

/** What the model says when a test scripted nothing. */
const NOTHING_SCRIPTED: Said = { text: "I have nothing to add." };

/** A member of an object of unknown shape. */
function memberOf(value: unknown, name: string): unknown {
  return typeof value === "object" && value !== null ? Reflect.get(value, name) : undefined;
}

/** The answer of the chat shape for something said. */
function answerFor(said: Said): unknown {
  const message =
    "text" in said
      ? { role: "assistant", content: said.text }
      : {
          role: "assistant",
          content: null,
          tool_calls: [
            {
              id: `call-${said.tool}`,
              type: "function",
              function: { name: said.tool, arguments: JSON.stringify(said.arguments) },
            },
          ],
        };
  return { choices: [{ message }], usage: { prompt_tokens: 10, completion_tokens: 10 } };
}

/** The texts of the turns of the person in a request of the chat shape. */
function personTurns(body: unknown): readonly string[] {
  const messages = memberOf(body, "messages");
  const list: readonly unknown[] = Array.isArray(messages) ? messages : [];
  return list.flatMap((each) => {
    const content = memberOf(each, "content");
    return memberOf(each, "role") === "user" && typeof content === "string" ? [content] : [];
  });
}

/** The text of a request body, read whole. */
async function textOf(request: IncomingMessage): Promise<string> {
  const parts: Uint8Array[] = [];
  for await (const part of request) {
    parts.push(part instanceof Uint8Array ? part : Buffer.from(String(part)));
  }
  return Buffer.concat(parts).toString("utf8");
}

/** Starts the fake model on a port of this computer that the system picks. */
export async function startFakeModel(): Promise<FakeModel> {
  const script: Said[] = [];
  const calls: (readonly string[])[] = [];
  const server = createServer((request, response: ServerResponse) => {
    void textOf(request).then((text) => {
      calls.push(personTurns(JSON.parse(text) as unknown));
      response.setHeader("content-type", "application/json");
      response.end(JSON.stringify(answerFor(script.shift() ?? NOTHING_SCRIPTED)));
    });
  });
  const port = await new Promise<number>((done, failed) => {
    server.once("error", failed);
    server.listen(0, "127.0.0.1", () => {
      const where = server.address();
      done(where !== null && typeof where === "object" ? where.port : 0);
    });
  });
  return {
    address: `http://127.0.0.1:${String(port)}/v1`,
    will: (...said) => {
      script.push(...said);
    },
    given: () => calls,
    close: async () => {
      server.closeAllConnections();
      await new Promise<void>((done) => {
        server.close(() => {
          done();
        });
      });
    },
  };
}
