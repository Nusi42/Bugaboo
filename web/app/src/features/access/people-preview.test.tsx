import { t } from "@bugaboo/messages";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import type { LevelTable, PeopleKindKey, PeopleLevelCode } from "./levels.ts";
import { PeoplePreview } from "./PeoplePreview.tsx";

/** A role that sees nobody: every kind of people information hidden. Words of the test, not text for people. */
const guest = { id: "r1", name: "Guest" };

const table: LevelTable<PeopleKindKey, PeopleLevelCode> = {
  levels: ["hidden", "role-only", "names"],
  roles: [guest],
  rows: (["assigned-to", "authors", "editing"] as const).map((subject) => ({
    subject,
    yours: "names",
    cells: [{ role: guest.id, level: "hidden", origin: { kind: "role-own" } }],
  })),
  truncated: false,
};

describe("priv_12 the preview of what a role sees", () => {
  it("writes someone in the middle of a sentence and Someone at its start, as every sentence of people does", () => {
    const markup = renderToStaticMarkup(<PeoplePreview table={table} role={guest.id} />);
    expect(markup).toContain(
      t("people-visibility.example-comment", { who: t("person.someone-inside") }),
    );
    expect(markup).toContain(t("people-visibility.example-editing", { who: t("person.someone") }));
  });
});
