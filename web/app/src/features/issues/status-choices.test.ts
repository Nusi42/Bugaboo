import { describe, expect, it } from "vitest";

import { choiceIdOf, choiceLabelOf } from "./status-choices.ts";

const open = { key: "open", name: "Open", group: "not-started" } as const;
const testing = { key: "testing", name: "Testing", group: "in-progress" } as const;
const done = { key: "done", name: "Done", group: "done" } as const;

describe("iss_09 the choices of a status change for many issues are told apart by their words", () => {
  it("names two changes to the same status differently", () => {
    const stop = choiceIdOf({ to: open, button: "Stop working" });
    const reopen = choiceIdOf({ to: open, button: "Reopen" });
    expect(stop).not.toBe(reopen);
  });

  it("names the same words to another status differently", () => {
    expect(choiceIdOf({ to: done, button: "Back" })).not.toBe(
      choiceIdOf({ to: testing, button: "Back" }),
    );
  });

  it("says the status only when the words alone would not tell the choices apart", () => {
    const offered = [
      { to: done, button: "Back" },
      { to: testing, button: "Back" },
      { to: open, button: "Reopen" },
    ];
    expect(choiceLabelOf({ to: open, button: "Reopen" }, offered)).toBe("Reopen");
    expect(choiceLabelOf({ to: done, button: "Back" }, offered)).toBe("Back (to Done)");
  });
});
