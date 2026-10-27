import type { Schema } from "@bugaboo/api-client";
import { describe, expect, it } from "vitest";

import { groupOfKey } from "./group-keys.ts";

type Group = Schema<"SearchGroupBody">;

describe("the group the server sends (UX 4.2, ADR-0005 item 9)", () => {
  it("keeps the filter the server wrote and never builds one", () => {
    const high: Group = {
      key: { type: "priority", value: "high" },
      issues: 3,
      filter: "priority is high",
    };
    expect(groupOfKey(high, "priority")).toMatchObject({
      filter: "priority is high",
      priority: "high",
    });
    const progress: Group = {
      key: { type: "status-group", value: "in-progress" },
      issues: 1,
      filter: "statusGroup is inProgress",
    };
    expect(groupOfKey(progress, "statusGroup")).toMatchObject({ statusGroup: "in-progress" });
  });

  it("names a group by a person or a label as the viewer sees it", () => {
    const person: Group = {
      key: { type: "person", value: { shown: "name", id: "p1", name: "Ana" } },
      issues: 1,
      filter: "assignedTo is Ana",
    };
    expect(groupOfKey(person, "assignedTo").name).toBe("Ana");
    const hidden: Group = {
      key: { type: "tag", value: { id: "t1", name: null } },
      issues: 1,
      filter: null,
    };
    expect(groupOfKey(hidden, "tags").filter).toBeUndefined();
  });

  it("names a group by an ID with the name in the filter the server wrote", () => {
    const project: Group = {
      key: { type: "project", value: "id-1" },
      issues: 2,
      filter: "project is WEB",
    };
    expect(groupOfKey(project, "project").name).toBe("WEB");
  });
});

describe("the places of the groups", () => {
  it("gives every group a name of its own, so two groups are never one", () => {
    const keys: Group[] = [
      { key: { type: "empty" }, issues: 1, filter: "x is empty" },
      { key: { type: "text", value: "a" }, issues: 1, filter: null },
      { key: { type: "text", value: "b" }, issues: 1, filter: null },
    ];
    expect(new Set(keys.map((key) => groupOfKey(key, "x").id)).size).toBe(keys.length);
  });
});
