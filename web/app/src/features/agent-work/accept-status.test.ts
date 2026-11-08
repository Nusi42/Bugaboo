import { describe, expect, it } from "vitest";

import { acceptChoices } from "./accept-status.ts";

const change = (key: string, name: string, group: "not-started" | "in-progress" | "done") => ({
  button: name,
  needs: [],
  to: { key, name, status_group: group },
});

describe("the statuses to accept into", () => {
  it("puts the statuses of the group Done first and names each status once", () => {
    const choices = acceptChoices([
      change("open", "Open", "not-started"),
      change("fixed", "Fixed", "done"),
      change("fixed", "Fixed", "done"),
      change("wontfix", "Won't fix", "done"),
    ]);
    expect(choices.map((choice) => choice.value)).toEqual(["fixed", "wontfix", "open"]);
    expect(choices.map((choice) => choice.proposed)).toEqual([true, false, false]);
  });

  it("offers nothing when the issue cannot change its status", () => {
    expect(acceptChoices(null)).toEqual([]);
  });
});
