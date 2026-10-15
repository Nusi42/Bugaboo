import { t } from "@bugaboo/messages";
import { Alert, Button, buttonLook, partOf, part } from "@bugaboo/ui";

import { formatAgo } from "../format.ts";

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
    <Alert>
      <div {...partOf(part.issueNote)}>
        <span>{found(when)}</span>
        <Button look={buttonLook.primary} onPress={onContinue}>
          {t("editing.draft-continue", { when })}
        </Button>
        <Button onPress={onDrop}>{t("editing.draft-drop")}</Button>
      </div>
    </Alert>
  );
}
