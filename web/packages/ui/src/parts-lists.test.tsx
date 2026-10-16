import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { listRowLook } from "./props.ts";
import { listColumns, type ListColumn } from "./list-columns.ts";
import { ListGrid, ListGroup, ListHead, ListRow } from "./ListGrid.tsx";

/** Words of the tests, not text for people. */
const text = {
  table: "Issues",
  title: "Title",
  group: "In progress",
  key: "Key",
} as const;
const group = { notStarted: "not-started", inProgress: "in-progress", done: "done" } as const;
const priority = { normal: "normal", critical: "critical" } as const;
const twelve = 12;

/** Every column of the theme file, with its own text: a column that is missing is a type error. */
const columnLabels = {
  select: "Select",
  status: "Status",
  key: "Key",
  type: "Type",
  title: "Title",
  labels: "Labels",
  priority: "Priority",
  person: "Assigned to",
  changed: "Changed",
  action: "Actions",
} as const satisfies Readonly<Record<ListColumn, string>>;

/** The columns of the cells of a markup, in the order they stand. */
function cellColumns(markup: string): string[] {
  return markup
    .split('data-part="list-row.cell" data-variant="')
    .slice(1)
    .map((piece) => piece.slice(0, piece.indexOf('"')));
}

describe("ux_4_2 the issue list is one grid with fixed columns", () => {
  it("names every column of the theme file once, in the order of the list", () => {
    expect(listColumns).toEqual([
      "select",
      "status",
      "key",
      "type",
      "title",
      "labels",
      "priority",
      "person",
      "changed",
      "action",
    ]);
  });

  it("the header and a row have the same cells in the same order, also when a cell is empty", () => {
    const head = renderToStaticMarkup(<ListHead labels={columnLabels} />);
    const row = renderToStaticMarkup(
      <ListRow
        group={group.inProgress}
        priority={priority.critical}
        cells={{ key: text.key, title: text.title }}
      />,
    );
    expect(cellColumns(head)).toEqual(listColumns);
    expect(cellColumns(row)).toEqual(listColumns);
    expect(head).toContain('role="columnheader"');
    expect(row).toContain('role="cell"');
    expect(row).toContain('data-state="group-in-progress priority-critical"');
  });

  it("is a table with a name", () => {
    const markup = renderToStaticMarkup(<ListGrid label={text.table}>{text.title}</ListGrid>);
    expect(markup).toContain('role="table"');
    expect(markup).toContain(`aria-label="${text.table}"`);
    expect(markup).toContain('data-part="list-grid"');
  });

  it("a column that holds only an icon is named for screen readers, the others are seen", () => {
    const markup = renderToStaticMarkup(<ListHead labels={columnLabels} />);
    expect(markup).toContain(`<span data-part="visually-hidden">${columnLabels.status}</span>`);
    expect(markup).toContain(`>${columnLabels.title}</div>`);
  });
});

describe("ux_4_2 a row says what it is", () => {
  it("a ticked row says so by its state", () => {
    const ticked = renderToStaticMarkup(
      <ListRow group={group.notStarted} priority={priority.normal} selected cells={{}} />,
    );
    expect(ticked).toContain('data-state="selected"');
    expect(ticked).toContain('aria-selected="true"');
  });

  it("a phone row is its own variant", () => {
    const phone = renderToStaticMarkup(
      <ListRow
        group={group.done}
        priority={priority.normal}
        variant={listRowLook.phone}
        cells={{}}
      />,
    );
    expect(phone).toContain('data-variant="phone"');
    expect(phone).toContain('data-state="group-done"');
  });

  it("a group of rows has a header with the group's icon, its name and its count", () => {
    const markup = renderToStaticMarkup(
      <ListGroup group={group.inProgress} label={text.group} count={twelve} />,
    );
    expect(markup).toContain('data-part="list-group"');
    expect(markup).toContain('data-part="status-icon"');
    expect(markup).toContain(text.group);
    expect(markup).toContain('data-part="list-group.count"');
    expect(markup).not.toContain("style=");
  });
});
