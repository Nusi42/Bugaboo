import { priorityWords, statusGroupWords } from "@bugaboo/api-client";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { Priority } from "./Priority.tsx";
import { StatusButton } from "./StatusButton.tsx";
import { StatusIcon } from "./StatusIcon.tsx";
import { StatusPill } from "./StatusPill.tsx";
import { doNothing } from "./do-nothing.ts";

/** Words of the tests, not text for people. */
const text = { inProgress: "In progress", critical: "Critical" } as const;
const group = { inProgress: "in-progress", done: "done" } as const;

describe("ux_r5 a status and a priority are never color alone", () => {
  it("the icon of every status group has its own part word and a name when it stands alone", () => {
    for (const word of statusGroupWords) {
      const named = renderToStaticMarkup(<StatusIcon group={word} label={text.inProgress} />);
      expect(named).toContain('data-part="status-icon"');
      expect(named).toContain(`data-variant="${word}"`);
      expect(named).toContain('role="img"');
      expect(named).toContain(`aria-label="${text.inProgress}"`);
      const decorating = renderToStaticMarkup(<StatusIcon group={word} />);
      expect(decorating).toContain('aria-hidden="true"');
      expect(decorating).not.toContain("aria-label");
    }
  });

  it("the pill and the big button show the group's icon and the word of the status", () => {
    const pill = renderToStaticMarkup(
      <StatusPill group={group.inProgress} label={text.inProgress} />,
    );
    expect(pill).toContain('data-part="status-pill"');
    expect(pill).toContain('data-variant="in-progress"');
    expect(pill).toContain('data-part="status-icon"');
    expect(pill).toContain(text.inProgress);
    const button = renderToStaticMarkup(
      <StatusButton group={group.done} label={text.inProgress} onPress={doNothing} />,
    );
    expect(button).toContain('data-part="status-button"');
    expect(button).toContain('data-variant="done"');
    expect(button).toContain('data-part="status-icon"');
    expect(button).toContain(text.inProgress);
    expect(pill + button).not.toContain("style=");
  });

  it("a priority has its icon, and its word on the screen or as its name", () => {
    for (const word of priorityWords) {
      const withWord = renderToStaticMarkup(<Priority priority={word} label={text.critical} />);
      expect(withWord).toContain('data-part="priority"');
      expect(withWord).toContain(`data-variant="${word}"`);
      expect(withWord).toContain('data-part="priority-icon"');
      expect(withWord).toContain(text.critical);
      const iconOnly = renderToStaticMarkup(
        <Priority priority={word} label={text.critical} showWord={false} />,
      );
      expect(iconOnly).toContain('data-part="priority-icon"');
      expect(iconOnly).toContain('role="img"');
      expect(iconOnly).toContain(`aria-label="${text.critical}"`);
    }
  });
});
