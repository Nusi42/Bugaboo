import type { Schema } from "@bugaboo/api-client";
import { describe, expect, it } from "vitest";

import { groupOfKey } from "./group-keys.ts";

type Key = Schema<"SearchGroupKeyBody">;

const priorityChoices = ["low", "normal", "high", "critical"];
const groupChoices = ["notStarted", "inProgress", "done"];

describe("the group of a key the server sends (UX 4.2, ADR-0005 item 9)", () => {
  it("is filtered by the word of the language, found among the choices of the field", () => {
    const high: Key = { type: "priority", value: "high" };
    expect(groupOfKey(high, priorityChoices).filter).toBe("high");
    const progress: Key = { type: "status-group", value: "in-progress" };
    expect(groupOfKey(progress, groupChoices)).toMatchObject({
      filter: "inProgress",
      statusGroup: "in-progress",
    });
  });

  it("is filtered by the name of a person, a label or a tag, as the viewer sees it", () => {
    const person: Key = { type: "person", value: { shown: "name", id: "p1", name: "Ana" } };
    expect(groupOfKey(person, []).filter).toBe("Ana");
    const label: Key = { type: "label", value: { id: "l1", name: "login" } };
    expect(groupOfKey(label, []).filter).toBe("login");
    const hidden: Key = { type: "tag", value: { id: "t1", name: null } };
    expect(groupOfKey(hidden, []).filter).toBeUndefined();
  });

  it("is filtered by emptiness when it has no value", () => {
    expect(groupOfKey({ type: "empty" }, []).filter).toEqual({ empty: true });
  });

  it("has no filter when the server names the group by an ID or a stored key only", () => {
    expect(groupOfKey({ type: "project", value: "id-1" }, []).filter).toBeUndefined();
    expect(groupOfKey({ type: "text", value: "in-progress" }, []).filter).toBeUndefined();
  });

  it("gives every group a name of its own, so two groups are never one", () => {
    const names = [
      groupOfKey({ type: "empty" }, []).id,
      groupOfKey({ type: "text", value: "a" }, []).id,
      groupOfKey({ type: "text", value: "b" }, []).id,
      groupOfKey({ type: "priority", value: "low" }, priorityChoices).id,
    ];
    expect(new Set(names).size).toBe(names.length);
  });
});
