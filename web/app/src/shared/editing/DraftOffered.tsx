import { t } from "@bugaboo/messages";

import { formatAgo } from "../format.ts";
import { DraftNote } from "./DraftNote.tsx";

export interface DraftOfferedProps {
  /** The sentence that says what the draft is of, with `{when}` already put in. */
  readonly found: (when: string) => string;
  /** When the draft was kept, as a moment in time. */
  readonly savedAt: string;
  readonly now: number;
  readonly onContinue: () => void;
  readonly onDrop: () => void;
}

/**
 * *Continue your draft from yesterday* (ISS-17, ISS-11): a text or a form was opened and the
 * person has a draft of it. The same offer for the description, a comment and a new issue.
 */
export function DraftOffered({ found, savedAt, now, onContinue, onDrop }: DraftOfferedProps) {
  const when = formatAgo(savedAt, now);
  return (
    <DraftNote
      text={found(when)}
      take={{ label: t("editing.draft-continue", { when }), onPress: onContinue }}
      drop={{ label: t("editing.draft-drop"), onPress: onDrop }}
    />
  );
}
