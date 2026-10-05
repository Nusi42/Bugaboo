import { t } from "@bugaboo/messages";
import { describe, expect, it } from "vitest";

import { liftSentencesOfPreview, sentencesOfPreview } from "./preview-sentences.ts";
import { asShortName } from "./short-name.ts";
import type { Preview } from "./use-projects.ts";

const web = { id: "w", short_name: "WEB", name: "Website" };
const cleo = { shown: "name", id: "c", name: "Cleo" } as const;
const nobody = { sees: false, held_back: false, roles: [] };
const viewer = { sees: true, held_back: false, roles: ["Viewer"] };
const none = { gains: [], loses: [], fields: [], people: [] };

/** A word of the vocabulary whose message this version of the app does not have. */
function word(code: string) {
  return { code, message_key: `no.such.${code}` };
}

/** A preview with the given people and lifts. */
function preview(people: Preview["people"], lifts: Preview["lifts"] = []): Preview {
  return {
    project: web,
    to: null,
    preview: "v1",
    blocked: lifts.length > 0,
    truncated: false,
    people,
    lifts,
  };
}

describe("prj_03 the preview in plain sentences", () => {
  it("says who gets access, with the roles they get", () => {
    const gained = preview([
      { person: cleo, projects: [{ project: web, now: nobody, after: viewer, ...none }] },
    ]);
    expect(sentencesOfPreview(gained)).toEqual([
      t("projects.line-gains", { person: "Cleo", project: "Website", roles: "Viewer" }),
    ]);
  });

  it("says who loses access and who changes role", () => {
    const changed = preview([
      { person: cleo, projects: [{ project: web, now: viewer, after: nobody, ...none }] },
      {
        person: { shown: "role", role: "Team member" },
        projects: [
          {
            project: web,
            now: viewer,
            after: { sees: true, held_back: false, roles: ["Viewer", "Guest"] },
            ...none,
          },
        ],
      },
    ]);
    expect(sentencesOfPreview(changed)).toEqual([
      t("projects.line-loses", { person: "Cleo", project: "Website" }),
      t("projects.line-roles", {
        person: t("projects.person-role", { role: "Team member" }),
        project: "Website",
        before: "Viewer",
        after: "Viewer, Guest",
      }),
    ]);
  });
});

describe("prj_03 how people are named in the preview", () => {
  it("names a person who may not be named as Someone and a removed person as a former user", () => {
    const hidden = preview([
      {
        person: { shown: "someone" },
        projects: [{ project: web, now: nobody, after: viewer, ...none }],
      },
      {
        person: { shown: "former", label: "K7Q2" },
        projects: [{ project: web, now: viewer, after: nobody, ...none }],
      },
    ]);
    const [first, second] = sentencesOfPreview(hidden);
    expect(first).toContain(t("projects.person-someone"));
    expect(second).toContain(t("projects.person-former", { label: "K7Q2" }));
  });

  it("says nothing for a change that changes nothing a person can read", () => {
    const same = preview([
      { person: cleo, projects: [{ project: web, now: viewer, after: viewer, ...none }] },
    ]);
    expect(sentencesOfPreview(same)).toEqual([]);
  });
});

describe("prj_03 the preview of a refused move and of what this version cannot say", () => {
  it("names the people who could take away a No access that applies to them", () => {
    const refused = preview([], [{ person: cleo, project: web }]);
    expect(liftSentencesOfPreview(refused)).toEqual([
      t("projects.line-lift", { person: "Cleo", project: "Website" }),
    ]);
  });

  it("falls back to the word itself for a message this version does not have", () => {
    const odd = preview([
      {
        person: cleo,
        projects: [
          {
            project: web,
            now: viewer,
            after: viewer,
            gains: [],
            loses: [],
            fields: [{ field: word("a-field"), before: word("hidden"), after: word("can-see") }],
            people: [],
          },
        ],
      },
    ]);
    expect(sentencesOfPreview(odd)).toEqual([
      t("projects.line-field", {
        person: "Cleo",
        project: "Website",
        field: "a-field",
        before: "hidden",
        after: "can-see",
      }),
    ]);
  });
});

describe("prj_01 the short name form", () => {
  it("turns the letters typed into capitals", () => {
    expect(asShortName("web2")).toBe("WEB2");
    expect(asShortName("Web")).toBe("WEB");
  });
});
