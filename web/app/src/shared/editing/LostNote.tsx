import { t } from "@bugaboo/messages";
import { Alert, alertTone, partOf, part } from "@bugaboo/ui";
import type { ReactNode } from "react";

import { personNameInside, someoneName } from "../person.ts";
import type { EditedThing } from "./edited-thing.ts";
import type { LostReason, Who } from "./long-text-model.ts";

export interface LostNoteProps {
  readonly reason: LostReason;
  /** Who took over or changed the text, if the screen can tell. */
  readonly by?: Who | undefined;
  readonly thing: EditedThing;
  /** The way on that is particular to the screen (*Compare and save*, *Try again*). */
  readonly children?: ReactNode;
}

/**
 * The note that tells why a text is *Not saved yet* and that it is kept (ISS-17, UX §9): the same
 * words for the description and a comment, with the way on that each screen has.
 */
export function LostNote({ reason, by, thing, children }: LostNoteProps) {
  const name = by === undefined ? someoneName("inside") : personNameInside(by);
  const what = t(thing);
  const why = {
    "ran-out": t("editing.lost-ran-out"),
    "taken-over": t("editing.lost-taken-over", { name }),
    "version-changed": t("editing.lost-changed", { name, thing: what }),
  } as const;
  return (
    <Alert tone={alertTone.warning}>
      <div {...partOf(part.issueNote)}>
        <span>{why[reason]}</span>
        {children}
      </div>
    </Alert>
  );
}
