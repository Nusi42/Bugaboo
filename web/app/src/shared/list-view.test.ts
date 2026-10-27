import { describe, expect, it } from "vitest";

import {
  builtInView,
  noGrouping,
  presentationOf,
  viewFromPresentation,
  viewOf,
  withView,
} from "./list-view.ts";
import { readSearch, searchLocation } from "./search-address.ts";

const project = builtInView({ everywhere: false, spacing: "comfortable" });
const everywhere = builtInView({ everywhere: true, spacing: "comfortable" });

describe("the view of a list that the address and the defaults give (UX 2.1, 4.2)", () => {
  it("is the columns of UX 4.2, grouped by status group, for a project's list", () => {
    expect(project.columns).toEqual(["status", "id", "title", "priority", "assignedTo", "changed"]);
    expect(project.groupBy).toBe("statusGroup");
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
