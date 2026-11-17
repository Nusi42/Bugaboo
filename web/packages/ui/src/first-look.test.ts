import { afterEach, describe, expect, it, vi } from "vitest";

import { defaultAppearance, firstLookOf, type Appearance } from "./appearance.ts";
import { firstLookPlaces, replayFirstLook, type FirstLook } from "./first-look.ts";
import { pageAttributes, variants } from "./tokens.ts";

/** An own theme of the dark side, with a sheet of the server. */
const own = { id: "theme-1", sheet: "/themes/theme-1.css?v=1", choice: "midnight/theme-1" };

/** A link as the fake head keeps it. */
interface FakeLink {
  id: string;
  rel: string;
  href: string;
  blocking: string | null;
}

/** A page frame and a head that only remember what is put on them. */
function fakeDocument() {
  const attributes = new Map<string, string>();
  const links: Readonly<FakeLink>[] = [];
  return {
    attributes,
    links,
    document: {
      documentElement: {
        setAttribute: (name: string, value: string) => void attributes.set(name, value),
      },
      createElement: () => {
        const link: FakeLink = { id: "", rel: "", href: "", blocking: null };
        return {
          ...link,
          setAttribute(name: string, value: string) {
            if (name === "blocking") {
              this.blocking = value;
            }
          },
        };
      },
      head: {
        append: (link: Readonly<FakeLink>) => void links.push(link),
      },
    },
  };
}

/** Puts a kept text, a device that is dark or light and a fake page where the script finds them. */
function prepare(kept: string | null, dark: boolean) {
  const fake = fakeDocument();
  vi.stubGlobal("localStorage", { getItem: () => kept });
  vi.stubGlobal("matchMedia", () => ({ matches: dark }));
  vi.stubGlobal("document", fake.document);
  return fake;
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("ux_01 the first look of the page frame", () => {
  it("keeps what the page frame carries on a light and on a dark device", () => {
    const appearance: Appearance = {
      ...defaultAppearance,
      light: "paper",
      dark: "midnight",
      darkOwn: own,
      spacing: "compact",
    };
    expect(firstLookOf(appearance)).toEqual({
      light: {
        attributes: {
          [pageAttributes.look]: "paper",
          [variants.compact.attribute]: variants.compact.value,
        },
        sheet: null,
      },
      dark: {
        attributes: {
          [pageAttributes.look]: "midnight",
          [pageAttributes.ownTheme]: own.id,
          [variants.compact.attribute]: variants.compact.value,
        },
        sheet: own.sheet,
      },
    } satisfies FirstLook);
  });
});

describe("ux_01 the script of the first look", () => {
  it("puts the look of the device's side back, with the sheet of an own theme that holds the paint", () => {
    const kept = JSON.stringify(
      firstLookOf({ ...defaultAppearance, dark: "midnight", darkOwn: own }),
    );
    const fake = prepare(kept, true);
    replayFirstLook(firstLookPlaces);
    expect(fake.attributes.get(pageAttributes.look)).toBe("midnight");
    expect(fake.attributes.get(pageAttributes.ownTheme)).toBe(own.id);
    expect(fake.links).toEqual([
      expect.objectContaining({
        id: firstLookPlaces.sheetId,
        rel: "stylesheet",
        href: own.sheet,
        blocking: "render",
      }),
    ]);
    const light = prepare(kept, false);
    replayFirstLook(firstLookPlaces);
    expect(light.attributes.get(pageAttributes.look)).toBe("daylight");
    expect(light.links).toEqual([]);
  });

  it("puts only the attributes of the look on the page and links only the server's sheets", () => {
    const kept = JSON.stringify({
      dark: {
        attributes: { [pageAttributes.look]: "dusk", onclick: "x", [pageAttributes.ownTheme]: 1 },
        sheet: "https://elsewhere.test/x.css",
      },
    });
    const fake = prepare(kept, true);
    replayFirstLook(firstLookPlaces);
    expect(Object.fromEntries(fake.attributes)).toEqual({ [pageAttributes.look]: "dusk" });
    expect(fake.links).toEqual([]);
    // An address that only starts like a sheet of the server is not linked either: the paint would wait for it.
    for (const sheet of [
      "/themes/../api/v1/me",
      "/themes/%2e%2e/api/v1/me.css",
      "//elsewhere.test/themes/x.css",
    ]) {
      const sneaky = prepare(JSON.stringify({ dark: { attributes: {}, sheet } }), true);
      replayFirstLook(firstLookPlaces);
      expect.soft(sneaky.links).toEqual([]);
    }
  });
});

describe("ux_01 the script of the first look with nothing good kept", () => {
  it("changes nothing when nothing is kept, the text is not JSON or the storage is blocked", () => {
    for (const kept of [null, "not json", '"dusk"']) {
      const fake = prepare(kept, true);
      replayFirstLook(firstLookPlaces);
      expect(fake.attributes.size).toBe(0);
    }
    const fake = prepare(null, true);
    vi.stubGlobal("localStorage", {
      getItem: () => {
        throw new DOMException("blocked", "SecurityError");
      },
    });
    replayFirstLook(firstLookPlaces);
    expect(fake.attributes.size).toBe(0);
  });
});
