import { describe, expect, it } from "vitest";

import {
  builtInView,
  noGrouping,
  presentationOf,
  viewFromPresentation,
  viewInWords,
  viewOf,
  withView,
} from "./list-view.ts";
import { readSearch, searchLocation } from "./search-address.ts";

const project = builtInView({ everywhere: false, spacing: "comfortable" });
const everywhere = builtInView({ everywhere: true, spacing: "comfortable" });

describe("an own field kept in the old words is found by its name (D3)", () => {
  it("keeps the column, the width and the grouping of an own field", () => {
    const old = {
      columns: ["id", "storyPoints", "assignedTo"],
      widths: { storyPoints: 120 },
      spacing: "comfortable",
      groupBy: "storyPoints",
    } as const;
    expect(viewInWords(old, ["id", "story points", "assigned to"])).toEqual({
      columns: ["id", "story points", "assigned to"],
      widths: { "story points": 120 },
      spacing: "comfortable",
      groupBy: "story points",
    });
    // A word that is no field of the person's stays as it is, and is left out later as unreadable.
    expect(viewInWords(old, []).columns).toEqual(["id", "storyPoints", "assigned to"]);
  });
});

describe("the words of the old language in an address or a saved search (D3)", () => {
  it("reads the words of the old language: an address or a saved search from before D3 still works", () => {
    const state = readSearch({
      pathname: "/p/WEB/issues",
      search: { cols: "id,assignedTo,statusGroup", widths: "assignedTo:200", group: "statusGroup" },
    });
    expect(viewOf(state, project)).toMatchObject({
      columns: ["id", "assigned to", "status group"],
      widths: { "assigned to": 200 },
      groupBy: "status group",
    });
    const saved = viewFromPresentation(
      {
        view: "list",
        columns: [{ field: "reportedBy", width: 90 }],
        spacing: "compact",
        group_by: "statusGroup",
      },
      project,
    );
    expect(saved).toMatchObject({
      columns: ["reported by"],
      widths: { "reported by": 90 },
      groupBy: "status group",
    });
  });
});

describe("the view of a list that the address and the defaults give (UX 2.1, 4.2)", () => {
  it("is the columns of UX 4.2, grouped by status group, for a project's list", () => {
    expect(project.columns).toEqual([
      "status",
      "id",
      "title",
      "priority",
      "assigned to",
      "changed",
    ]);
    expect(project.groupBy).toBe("status group");
    expect(project.spacing).toBe("comfortable");
    expect(project.widths).toEqual({});
  });

  it("has a Project column after the ID on the Search page, and nowhere else", () => {
    expect(everywhere.columns.slice(0, 3)).toEqual(["status", "id", "project"]);
    expect(project.columns).not.toContain("project");
  });

  it("takes from the address what it says and from the defaults what it leaves out", () => {
    const state = readSearch({
      pathname: "/p/WEB/issues",
      search: { cols: "id,title", widths: "title:300", spacing: "compact", group: "priority" },
    });
    expect(viewOf(state, project)).toEqual({
      columns: ["id", "title"],
      widths: { title: 300 },
      spacing: "compact",
      groupBy: "priority",
    });
    expect(viewOf(readSearch({ search: {} }), project)).toEqual(project);
  });

  it("reads the word of no grouping as no grouping", () => {
    const state = readSearch({ search: { group: noGrouping } });
    expect(viewOf(state, project).groupBy).toBeNull();
  });
});

describe("the view written back into the address", () => {
  it("writes into the address only what differs from the defaults, and keeps the text", () => {
    const state = readSearch({ pathname: "/p/WEB/issues", search: { s: "status is Open" } });
    const same = withView(state, project, project);
    expect(searchLocation(same).search).toEqual({ s: "status is Open" });
    const changed = withView(
      state,
      {
        ...project,
        columns: ["id", "title"],
        widths: { title: 300 },
        spacing: "compact",
        groupBy: null,
      },
      project,
    );
    expect(searchLocation(changed).search).toEqual({
      s: "status is Open",
      cols: "id,title",
      widths: "title:300",
      spacing: "compact",
      group: noGrouping,
    });
  });

  it("goes back to the defaults by leaving the parts out", () => {
    const state = readSearch({ search: { s: "a", cols: "id", group: "priority" } });
    expect(searchLocation(withView(state, project, project)).search).toEqual({ s: "a" });
  });
});

describe("the view as a saved search keeps it", () => {
  const view = {
    columns: ["id", "title"],
    widths: { title: 300 },
    spacing: "compact",
    groupBy: "priority",
  } as const;

  it("is written with the widths on the columns and a null for no grouping", () => {
    expect(presentationOf({ ...view, groupBy: null })).toEqual({
      view: "list",
      columns: [
        { field: "id", width: null },
        { field: "title", width: 300 },
      ],
      spacing: "compact",
      group_by: null,
    });
  });

  it("is read back, and a saved search with no columns has the columns of the list it is opened in", () => {
    const saved = presentationOf(view);
    expect(viewFromPresentation(saved, project)).toEqual(view);
    expect(viewFromPresentation({ ...saved, columns: [] }, everywhere).columns).toEqual(
      everywhere.columns,
    );
  });
});
