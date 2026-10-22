import { describe, expect, it } from "vitest";

import { personName, personNameInside } from "./person.ts";

describe("a person seen by their role only is Someone, with the role (glossary)", () => {
  const person = { shown: "role", role: "Team member" } as const;
  it("reads Someone (Team member) at the start and someone (Team member) inside", () => {
    expect(personName(person)).toBe("Someone (Team member)");
    expect(personNameInside(person)).toBe("someone (Team member)");
  });
});
