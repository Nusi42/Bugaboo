import { describe, expect, it } from "vitest";

import { chooseColumns, cellOf, listWord } from "../../shared/list-columns.ts";
import { fieldsFor, foundLineOf } from "./found-issues.ts";

const item = {
  id: "i1",
  readable_id: "WEB-1",
  title: "Plan",
  status: { key: "open", name: "Open", status_group: "not-started" },
  priority: "normal",
  start_date: "2026-11-03",
  due_date: "2026-11-07",
} as const;

describe("plan_01 the two dates are columns of the list", () => {
  it("asks the server for a date only when a column shows it", () => {
    expect(fieldsFor([listWord.dueDate])).toContain("due_date");
    expect(fieldsFor([listWord.dueDate])).not.toContain("start_date");
    expect(fieldsFor([listWord.startDate, listWord.dueDate])).toContain("start_date");
    expect(fieldsFor([])).not.toContain("due_date");
  });

  it("reads the days of a row", () => {
    const line = foundLineOf(item);
    expect(line?.startDate).toBe("2026-11-03");
    expect(line?.dueDate).toBe("2026-11-07");
    expect(line).toBeDefined();
  });

  it("is a plain value among the columns a person may choose", () => {
    expect(cellOf(listWord.dueDate)).toBe("value");
    expect(chooseColumns([listWord.dueDate], [listWord.dueDate])).toContain(listWord.dueDate);
  });
});
