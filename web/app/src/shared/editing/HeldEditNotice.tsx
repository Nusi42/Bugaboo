import { secondsPerMinute } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, partOf, part } from "@bugaboo/ui";
import type { ReactNode } from "react";

import { formatSince, secondsSince } from "../format.ts";
import { personName, personNameInside } from "../person.ts";
import { useNow } from "../use-now.ts";
import type { EditedThing } from "./edited-thing.ts";
import type { HeldBySomebody } from "./long-text-model.ts";

/** Who is editing and since when; *right now* for less than a minute. */
function saying(held: HeldBySomebody, what: string, now: number): string {
  const name = personName(held.holder);
  return secondsSince(held.since, now) < secondsPerMinute
    ? t("editing.is-editing-now", { name, thing: what })
    : t("editing.is-editing", { name, thing: what, since: formatSince(held.since, now) });
}

export interface HeldEditNoticeProps {
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
 * (ISS-17, UX §9): the sentence on top, the buttons in one row under it (one column on a phone).
 * One note for the description, a comment, a layout and a theme; a lock the person holds in another
 * window of their own says so and offers no asking.
 */
export function HeldEditNotice({
  held,
  thing,
  onAskToFinish,
  onEditAnyway,
  children,
}: HeldEditNoticeProps) {
  const now = useNow();
  const what = t(thing);
  return (
    <Alert tone={alertTone.info}>
      <div {...partOf(part.editHeld)}>
        <span>
          {held.yours_elsewhere ? t("editing.elsewhere", { thing: what }) : saying(held, what, now)}
        </span>
        <div {...partOf(part.editHeldActions)}>
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
      </div>
    </Alert>
  );
}
