import { priorityWords, statusGroupWords } from "@bugaboo/api-client";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { partOf } from "./parts.ts";
import { Priority } from "./Priority.tsx";
import { rowStates } from "./row-state.ts";
import { StatusButton } from "./StatusButton.tsx";
import { StatusIcon } from "./StatusIcon.tsx";
import { StatusPill } from "./StatusPill.tsx";
import { part, partWords } from "./tokens.ts";

/** Words of the tests, not text for people. */
const text = { inProgress: "In progress", critical: "Critical" } as const;
const group = { notStarted: "not-started", inProgress: "in-progress", done: "done" } as const;
const priority = { low: "low", normal: "normal", high: "high", critical: "critical" } as const;

const nothing = (): void => undefined;

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
      <StatusButton group={group.done} label={text.inProgress} onPress={nothing} />,
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

describe("ux_r4 a row carries the state of its status group and of its priority", () => {
  it("only the groups in progress and done are tinted, only critical and high are edged", () => {
    expect(rowStates({ group: group.notStarted, priority: priority.normal })).toEqual([]);
    expect(rowStates({ group: group.inProgress, priority: priority.low })).toEqual([
      "group-in-progress",
    ]);
    expect(rowStates({ group: group.done, priority: priority.high })).toEqual([
      "group-done",
      "priority-high",
    ]);
    expect(rowStates({ group: group.inProgress, priority: priority.critical })).toEqual([
      "group-in-progress",
      "priority-critical",
    ]);
  });

  it("every row that can carry them has the words in the theme file", () => {
    const words = rowStates({ group: group.done, priority: priority.critical });
    const rows = [partWords[part.listRow], partWords[part.issueRow], partWords[part.timelineBar]];
    for (const row of rows) {
      for (const word of words) {
        expect(row.state).toContain(word);
      }
    }
    const attributes = partOf(part.listRow, {
      states: rowStates({ group: group.done, priority: priority.high }),
    });
    expect(attributes["data-state"]).toBe("group-done priority-high");
  });
});
