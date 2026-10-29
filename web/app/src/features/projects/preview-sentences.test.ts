import { t } from "@bugaboo/messages";
import { describe, expect, it } from "vitest";

import { keyedLiftSentencesOfPreview, personSentencesOfPreview } from "./preview-sentences.ts";
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

/** Every sentence of the preview, in order, as the long preview shows them. */
function sentencesOfPreview(of: Preview): readonly string[] {
  return personSentencesOfPreview(of).flatMap((group) => group.lines.map((line) => line.text));
}

/** The keys of the sentences of the preview, each made of its person's key and its own. */
function keysOfPreview(of: Preview): readonly string[] {
  return personSentencesOfPreview(of).flatMap((group) =>
    group.lines.map((line) => `${group.key}:${line.key}`),
  );
}

/** A preview with the given people and lifts. */
function preview(people: Preview["people"], lifts: Preview["lifts"] = []): Preview {
  return {
    project: web,
    to: null,
    preview: "v1",
    blocked: lifts.length > 0,
    blocked_out_of_sight: false,
    truncated: false,
    summary: { will_change: people.length, skipped: 0 },
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
        person: t("person.role", { role: "Team member" }),
        project: "Website",
        before: "Viewer",
        after: "Viewer and Guest",
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
        person: { shown: "former", code: "K7Q2" },
        projects: [{ project: web, now: viewer, after: nobody, ...none }],
      },
    ]);
    const [first, second] = sentencesOfPreview(hidden);
    expect(first).toContain(t("person.someone"));
    expect(second).toContain(t("person.former", { label: "K7Q2" }));
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
    expect(keyedLiftSentencesOfPreview(refused).map((line) => line.text)).toEqual([
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

describe("prj_03 what changes in the same way is said once", () => {
  it("puts the fields that go from one level to another into one sentence, and keeps the others apart", () => {
    const many = preview([
      {
        person: cleo,
        projects: [
          {
            project: web,
            now: viewer,
            after: viewer,
            gains: [],
            loses: [],
            fields: [
              { field: word("title"), before: word("can-change"), after: word("hidden") },
              { field: word("status"), before: word("can-change"), after: word("hidden") },
              { field: word("labels"), before: word("can-see"), after: word("hidden") },
            ],
            people: [],
          },
        ],
      },
    ]);
    expect(sentencesOfPreview(many)).toEqual([
      t("projects.line-fields-many", {
        person: "Cleo",
        project: "Website",
        before: "can-change",
        after: "hidden",
        fields: "title and status",
      }),
      t("projects.line-field", {
        person: "Cleo",
        project: "Website",
        field: "labels",
        before: "can-see",
        after: "hidden",
      }),
    ]);
  });
});

describe("prj_03 the keys of the sentences", () => {
  it("name whose change in which project a sentence is, not the words of the sentence", () => {
    const same = [
      { person: cleo, projects: [{ project: web, now: nobody, after: viewer, ...none }] },
      { person: cleo, projects: [{ project: web, now: nobody, after: viewer, ...none }] },
    ];
    const keys = keysOfPreview(preview(same));
    // Two people with the same sentence are two lines, each with a key of its own.
    expect(keys).toEqual(["0:w:0", "1:w:0"]);
    expect(new Set(keys).size).toBe(keys.length);
    const lifts = keyedLiftSentencesOfPreview(
      preview(
        [],
        [
          { person: cleo, project: web },
          { person: cleo, project: { ...web, id: "x" } },
        ],
      ),
    ).map((line) => line.key);
    expect(lifts).toEqual(["0:w", "1:x"]);
  });
});

describe("prj_01 the short name form", () => {
  it("turns the letters typed into capitals", () => {
    expect(asShortName("web2")).toBe("WEB2");
    expect(asShortName("Web")).toBe("WEB");
  });
});

describe("m2c_f80 the preview grouped by person for the long preview", () => {
  it("makes a group for each person with a sentence and leaves out a person with none", () => {
    const groups = personSentencesOfPreview(
      preview([
        { person: cleo, projects: [{ project: web, now: nobody, after: viewer, ...none }] },
        {
          person: { shown: "former", code: "K7Q2" },
          projects: [{ project: web, now: viewer, after: viewer, ...none }],
        },
      ]),
    );
    expect(groups.map((group) => group.person)).toEqual(["Cleo"]);
    expect(groups[0]?.lines.map((line) => line.text)).toEqual([
      t("projects.line-gains", { person: "Cleo", project: "Website", roles: "Viewer" }),
    ]);
  });
});
