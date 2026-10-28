import { fakeServer, settingKeys } from "@bugaboo/api-client";
import { parseWindowSizes } from "@bugaboo/ui";
import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  changeMySettings,
  forgetSeenVersions,
  rereadsMax,
  saveProgress,
} from "./change-my-settings.ts";
import { currentMySettings, mySettingsKey, type MySettings, type Setting } from "./my-settings.ts";
import { queryClient } from "./query-client.ts";
import { changeForSize } from "./window-sizes.ts";

/** The server of these tests: the one setting of the sizes, at a version, and what was asked. */
const server = vi.hoisted(() => ({
  sizes: "{}",
  version: 0,
  puts: [] as { readonly version: number | undefined; readonly sizes: string }[],
  reads: 0,
  /** Another tab changes the setting each time this page reads it, for a test of a lost race. */
  rival: false,
  respond: undefined as undefined | ((method: string, body: string) => Response),
}));

vi.mock("./api.ts", async () => {
  const { createApi } = await import("@bugaboo/api-client");
  return {
    api: createApi({
      baseUrl: "http://bugaboo.test",
      csrfToken: () => "token",
      fetch: fakeServer(
        (method, _address, body) =>
          server.respond?.(method, body) ?? new Response(null, { status: 500 }),
      ),
    }),
  };
});

const SIZES_KEY = settingKeys.windowSizes;
const small = { width: 400, height: 300 };

/** The setting of the sizes as the server holds it now. */
function sizesSetting(): Setting {
  return {
    key: SIZES_KEY,
    name_key: "app.name",
    help_key: "app.name",
    kind: "text",
    format: "window-sizes",
    choices: [],
    levels: ["person"],
    lockable: false,
    default: "{}",
    value: server.sizes,
    source: { kind: "person" },
    own: true,
    locked: false,
    version: server.version,
  };
}

function json(body: unknown, status = 200, type = "application/json"): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": type },
  });
}

/** Another tab sets the size of a form: the server moves on one version. */
function otherTabSets(form: string): void {
  server.sizes = JSON.stringify({
    ...parseWindowSizes(server.sizes),
    [form]: small,
  });
  server.version += 1;
}

/** The body of a change, as the page sent it. */
function sentOf(text: string): { value: string; version?: number } {
  const sent: unknown = JSON.parse(text);
  if (typeof sent !== "object" || sent === null || !("value" in sent)) {
    throw new Error("not a change");
  }
  const version = "version" in sent && typeof sent.version === "number" ? sent.version : undefined;
  return { value: String(sent.value), ...(version === undefined ? {} : { version }) };
}

/** The forms that have a size in the text of the setting. */
function formsOf(text: string): string[] {
  return Object.keys(parseWindowSizes(text));
}

/** What the server of these tests answers to a request: its method and the text of its body. */
function answer(method: string, text: string): Response {
  if (method === "GET") {
    server.reads += 1;
    const seen = json({ settings: [sizesSetting()] });
    if (server.rival) {
      // Another tab changes it right after this page has read it.
      otherTabSets(`rival-${String(server.version)}`);
    }
    return seen;
  }
  const body = sentOf(text);
  server.puts.push({ version: body.version, sizes: body.value });
  if (body.version !== undefined && body.version !== server.version) {
    return json(
      {
        type: "edit.version-changed",
        title: "x",
        status: 409,
        code: "edit.version-changed",
        message_key: "error.edit.version-changed",
        params: {},
      },
      409,
      "application/problem+json",
    );
  }
  server.sizes = body.value;
  server.version += 1;
  return json(sizesSetting());
}

/** The text of the sizes that the page shows. */
function textOfCache(): string {
  const value = currentMySettings()?.settings[0]?.value;
  return typeof value === "string" ? value : "{}";
}

/** What the page has read of the settings: the sizes at the version the server had then. */
function pageHasRead(): MySettings {
  const settings = { settings: [sizesSetting()] };
  queryClient.setQueryData(mySettingsKey(), settings);
  return settings;
}

describe("m5_f97 a change of a person's setting made from an old version", () => {
  beforeEach(() => {
    queryClient.clear();
    forgetSeenVersions();
    Object.assign(server, {
      sizes: "{}",
      version: 0,
      puts: [],
      reads: 0,
      rival: false,
    });
    server.respond = answer;
  });

  it("is sent with the version the page read, and needs no second try when nobody else changed", async () => {
    server.sizes = JSON.stringify({ a: small });
    server.version = 1;
    const read = pageHasRead();
    await changeMySettings([changeForSize(read, "b", small)]);
    expect(server.puts.map((put) => put.version)).toEqual([1]);
    expect(server.reads).toBe(0);
    expect(parseWindowSizes(server.sizes)).toEqual({ a: small, b: small });
    expect(saveProgress.read().state).toBe("saved");
  });

  it("reads the setting again and puts only its own form's size into what another tab set", async () => {
    server.sizes = JSON.stringify({ a: small });
    server.version = 1;
    const read = pageHasRead();
    // Another tab sets the size of another form after this page read the setting.
    otherTabSets("other-tab");
    await changeMySettings([changeForSize(read, "mine", { width: 900, height: 700 })]);
    expect(server.puts.map((put) => put.version)).toEqual([1, 2]);
    expect(server.reads).toBe(1);
    expect(parseWindowSizes(server.sizes)).toEqual({
      a: small,
      "other-tab": small,
      mine: { width: 900, height: 700 },
    });
    expect(saveProgress.read().state).toBe("saved");
  });
});

describe("m5_f97 changes of one tab that follow each other", () => {
  beforeEach(() => {
    queryClient.clear();
    forgetSeenVersions();
    Object.assign(server, {
      sizes: "{}",
      version: 0,
      puts: [],
      reads: 0,
      rival: false,
    });
    server.respond = answer;
  });

  it("sends the second of two quick changes from the version the first one got", async () => {
    server.version = 4;
    const read = pageHasRead();
    const first = changeMySettings([changeForSize(read, "a", small)]);
    // The page shows the first change at once, so the second one is made from what it shows.
    await vi.waitFor(() => {
      expect(formsOf(textOfCache())).toContain("a");
    });
    const second = changeMySettings([changeForSize(currentMySettings(), "b", small)]);
    await Promise.all([first, second]);
    expect(server.puts.map((put) => put.version)).toEqual([4, 5]);
    expect(server.reads).toBe(0);
    expect(formsOf(server.sizes).toSorted()).toEqual(["a", "b"]);
  });

  it("gives up after a few tries when the others keep winning, and says it was not saved", async () => {
    const read = pageHasRead();
    server.rival = true;
    otherTabSets("first");
    await changeMySettings([changeForSize(read, "mine", small)]);
    expect(server.puts).toHaveLength(rereadsMax + 1);
    expect(saveProgress.read().state).toBe("not-saved");
    expect(formsOf(server.sizes)).not.toContain("mine");
  });
});
