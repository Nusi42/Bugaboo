import type { Schema } from "@bugaboo/api-client";
import { describe, expect, it } from "vitest";

import { formatCalendarDay } from "../../shared/format.ts";
import { groupOfKey, groupsOfKeys } from "./group-keys.ts";

type Group = Schema<"SearchGroupBody">;

describe("the group the server sends (UX 4.2, ADR-0005 item 9)", () => {
  it("keeps the filter the server wrote and never builds one", () => {
    const high: Group = {
      key: { type: "priority", value: "high" },
      issues: 3,
      filter: "priority is high",
      name: null,
    };
    expect(groupOfKey(high)).toMatchObject({
      filter: "priority is high",
      priority: "high",
    });
    const progress: Group = {
      key: { type: "status-group", value: "in-progress" },
      issues: 1,
      filter: "statusGroup is inProgress",
      name: null,
    };
    expect(groupOfKey(progress)).toMatchObject({ statusGroup: "in-progress" });
  });

  it("names a group by a person or a label as the viewer sees it", () => {
    const person: Group = {
      key: { type: "person", value: { shown: "name", id: "p1", name: "Ana" } },
      issues: 1,
      filter: "assignedTo is Ana",
      name: "Ana",
    };
    expect(groupOfKey(person).name).toBe("Ana");
    const hidden: Group = {
      key: { type: "tag", value: { id: "t1", name: null } },
      issues: 1,
      filter: null,
      name: null,
    };
    expect(groupOfKey(hidden).filter).toBeUndefined();
  });

  it("names a group by an ID with the name the server gave it", () => {
    const project: Group = {
      key: { type: "project", value: "id-1" },
      issues: 2,
      filter: "project is WEB",
      name: "WEB",
    };
    expect(groupOfKey(project).name).toBe("WEB");
    expect(groupOfKey({ ...project, name: null }).name).toBeUndefined();
  });
});

describe("the places of the groups", () => {
  it("gives every group a name of its own, so two groups are never one", () => {
    const keys: Group[] = [
      { key: { type: "empty" }, issues: 1, filter: "x is empty", name: null },
      { key: { type: "text", value: "a" }, issues: 1, filter: null, name: null },
      { key: { type: "text", value: "b" }, issues: 1, filter: null, name: null },
      {
        key: { type: "person", value: { shown: "role", role: "Developer" } },
        issues: 1,
        filter: null,
        name: null,
      },
      {
        key: { type: "person", value: { shown: "role", role: "Developer" } },
        issues: 1,
        filter: null,
        name: null,
      },
    ];
    expect(new Set(groupsOfKeys(keys).map((group) => group.id)).size).toBe(keys.length);
  });
});

const whole = (value: number): Group => ({
  key: { type: "whole", value },
  issues: 1,
  filter: null,
  name: null,
});
const column = (kind: Schema<"FieldType">) => ({ id: "f1", kind, choices: {} });

describe("a group of whole numbers", () => {
  it("is worded as the issue page words the field", () => {
    expect(groupOfKey(whole(1), column("yes-no")).name).toBe("Yes");
    expect(groupOfKey(whole(5400), column("duration")).name).toBe("1h 30m");
    expect(groupOfKey(whole(1_500_000), column("decimal")).name).toMatch(/1[.,]5/u);
    expect(groupOfKey(whole(20_000), column("date")).name).toBe(formatCalendarDay("2024-10-04"));
    expect(groupOfKey(whole(7)).name).toBe("7");
  });
});
