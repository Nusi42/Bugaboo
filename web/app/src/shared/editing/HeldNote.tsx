import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, partOf, part } from "@bugaboo/ui";
import type { ReactNode } from "react";

import { formatSince } from "../format.ts";
import { personName, personNameInside } from "../person.ts";
import { useNow } from "../use-now.ts";
import type { EditedThing } from "./edited-thing.ts";
import type { HeldBySomebody } from "./long-text-model.ts";

export interface HeldNoteProps {
  readonly held: HeldBySomebody;
  readonly thing: EditedThing;
  /** *Ask … to finish*, offered when somebody else holds the lock; left out where the screen has none. */
  readonly onAskToFinish?: () => void;
  /** *Edit anyway*, offered when the person may take the editing over. */
  readonly onEditAnyway?: () => void;
  /** The way out that is particular to the screen (*Try again*, *Close*). */
  readonly children?: ReactNode;
}

/**
 * Somebody else is editing a text or a layout: who, since when, and the things the person can do
 * (ISS-17, UX §9). One note for the description, a comment and a layout; a lock the person holds
 * in another window of their own says so and offers no asking.
 */
export function HeldNote({ held, thing, onAskToFinish, onEditAnyway, children }: HeldNoteProps) {
  const now = useNow();
  const what = t(thing);
  return (
    <Alert tone={alertTone.info}>
      <div {...partOf(part.issueNote)}>
        <span>
          {held.yours_elsewhere
            ? t("editing.elsewhere", { thing: what })
            : t("editing.is-editing", {
                name: personName(held.holder),
                thing: what,
                since: formatSince(held.since, now),
              })}
        </span>
        {held.yours_elsewhere || onAskToFinish === undefined ? null : (
          <Button onPress={onAskToFinish}>
            {t("editing.ask-to-finish", { name: personNameInside(held.holder) })}
          </Button>
        )}
        {held.can_edit_anyway && onEditAnyway !== undefined ? (
          <Button onPress={onEditAnyway}>{t("editing.edit-anyway")}</Button>
        ) : null}
        {children}
      </div>
    </Alert>
  );
}
