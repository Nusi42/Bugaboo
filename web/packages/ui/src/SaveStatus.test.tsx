import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { keepsRoom } from "./props.ts";
import { SaveStatus, type SaveState } from "./SaveStatus.tsx";

/** Words of the tests, not text for people. */
const labels = { saving: "Saving", saved: "Saved", "not-saved": "Not saved yet" } as const;

/** The place of the words as the page has it. */
function drawn(state: SaveState, given: Partial<typeof labels> = labels, keeps = true): string {
  return renderToStaticMarkup(
    <SaveStatus state={state} labels={given} {...(keeps ? { variant: keepsRoom } : {})} />,
  );
}

/** The words that keep the room: unseen shapes, each an empty element. */
const roomWords =
  /<span data-part="save-status.room" data-words="[^"]*" aria-hidden="true"><\/span>/gu;

describe("ux_6_4 the words of a change keep their room and are heard once", () => {
  it("has exactly one status region, in every state and with or without the room", () => {
    for (const state of ["idle", "saving", "saved", "not-saved"] as const) {
      for (const keeps of [true, false]) {
        expect(drawn(state, labels, keeps).match(/role="status"/gu)).toHaveLength(1);
      }
    }
  });

  it("keeps every word as an unread shape with no text of its own", () => {
    const html = drawn("saved");
    expect(html.match(roomWords)).toHaveLength(Object.keys(labels).length);
    expect(html).toContain('data-variant="keeps-room"');
  });

  it("shows the words of the state now, and none before the first change", () => {
    expect(drawn("saving")).toContain(`role="status">${labels.saving}</span>`);
    expect(drawn("not-saved")).toContain(`role="status">${labels["not-saved"]}</span>`);
    expect(drawn("idle")).toContain('role="status"></span>');
  });

  it("keeps no room for words the place never shows", () => {
    const html = drawn("saved", { saving: labels.saving, saved: labels.saved });
    expect(html.match(roomWords)).toHaveLength(2);
    expect(html).not.toContain(labels["not-saved"]);
  });
});
