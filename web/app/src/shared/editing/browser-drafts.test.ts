import type { DraftTarget } from "@bugaboo/api-client";
import { draftKeepDays, msPerSecond, secondsPerDay } from "@bugaboo/api-client";
import type { KeyStore } from "@bugaboo/ui";
import { describe, expect, it } from "vitest";

import {
  addressOf,
  forgetBrowserDraft,
  forgetBrowserDrafts,
  readBrowserDraft,
  writeBrowserDraft,
} from "./browser-drafts.ts";

/** A storage that keeps its items in a map. */
function memory(): { readonly store: () => KeyStore; readonly items: Map<string, string> } {
  const items = new Map<string, string>();
  const store: KeyStore = {
    getItem: (key) => items.get(key) ?? null,
    setItem: (key, value) => {
      items.set(key, value);
    },
    removeItem: (key) => {
      items.delete(key);
    },
  };
  return { store: () => store, items };
}

const description: DraftTarget = { kind: "issue", id: "i1", part: "description" };
const newIssue: DraftTarget = { kind: "issue", project: "p1" };
const now = Date.parse("2026-10-10T10:00:00Z");

describe("iss_17 the browser keeps a draft for each person and each target", () => {
  it("keeps what was written and gives it back to the same person only", () => {
    const { store } = memory();
    writeBrowserDraft("ana", description, { text: "My words", baseVersion: 3 }, now, store);
    expect(readBrowserDraft("ana", description, now, store)?.text).toBe("My words");
    expect(readBrowserDraft("ana", description, now, store)?.baseVersion).toBe(3);
    expect(readBrowserDraft("ben", description, now, store)).toBeNull();
    expect(readBrowserDraft("ana", newIssue, now, store)).toBeNull();
  });

  it("names a part of a record and a new record differently", () => {
    expect(addressOf("ana", description)).not.toBe(addressOf("ana", newIssue));
    expect(addressOf("ana", description)).toBe("ana:edit:issue:i1:description");
    expect(addressOf("ana", newIssue)).toBe("ana:new:issue:p1");
  });

  it("drops a draft past its keep period, and keeps the others", () => {
    const { store } = memory();
    writeBrowserDraft("ana", description, { text: "Old", baseVersion: 1 }, now, store);
    const later = now + (draftKeepDays + 1) * secondsPerDay * msPerSecond;
    writeBrowserDraft("ana", newIssue, { text: "New", baseVersion: null }, later, store);
    expect(readBrowserDraft("ana", description, later, store)).toBeNull();
    expect(readBrowserDraft("ana", newIssue, later, store)?.text).toBe("New");
  });

  it("throws one draft away and then all of them", () => {
    const { store, items } = memory();
    writeBrowserDraft("ana", description, { text: "A", baseVersion: 1 }, now, store);
    writeBrowserDraft("ana", newIssue, { text: "B", baseVersion: null }, now, store);
    forgetBrowserDraft("ana", description, now, store);
    expect(readBrowserDraft("ana", description, now, store)).toBeNull();
    expect(readBrowserDraft("ana", newIssue, now, store)?.text).toBe("B");
    forgetBrowserDraft("ana", newIssue, now, store);
    expect(items.size).toBe(0);
    writeBrowserDraft("ana", newIssue, { text: "C", baseVersion: null }, now, store);
    forgetBrowserDrafts(store);
    expect(items.size).toBe(0);
  });

  it("reads nothing from a storage that holds something else", () => {
    const { store, items } = memory();
    items.set("bugaboo.drafts", "not json");
    expect(readBrowserDraft("ana", description, now, store)).toBeNull();
    items.set("bugaboo.drafts", JSON.stringify({ "ana:edit:issue:i1:description": { text: 1 } }));
    expect(readBrowserDraft("ana", description, now, store)).toBeNull();
  });
});

describe("iss_17 the browser keeps no draft for nobody", () => {
  it("keeps nothing for nobody: the person is not known yet", () => {
    const { store, items } = memory();
    writeBrowserDraft(null, description, { text: "A", baseVersion: 1 }, now, store);
    expect(items.size).toBe(0);
    writeBrowserDraft("ana", description, { text: "B", baseVersion: 1 }, now, store);
    expect(readBrowserDraft(null, description, now, store)).toBeNull();
    forgetBrowserDraft(null, description, now, store);
    expect(readBrowserDraft("ana", description, now, store)?.text).toBe("B");
  });
});
