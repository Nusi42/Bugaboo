import { describe, expect, it } from "vitest";

import { choiceIdOf, statusKeyOfChoice } from "./status-choices.ts";

const open = { key: "open", name: "Open", group: "not-started" } as const;

describe("iss_09 the choices of a status change for many issues are told apart by their words", () => {
  it("names two changes to the same status differently", () => {
    const stop = choiceIdOf({ to: open, button: "Stop working" });
    const reopen = choiceIdOf({ to: open, button: "Reopen" });
    expect(stop).not.toBe(reopen);
  });

  it("gives back the key of the status the choice leads to, whatever the words say", () => {
    expect(statusKeyOfChoice(choiceIdOf({ to: open, button: "Back | to work" }))).toBe("open");
  });
});
