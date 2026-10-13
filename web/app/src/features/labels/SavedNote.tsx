import { t } from "@bugaboo/messages";
import { Alert, alertTone } from "@bugaboo/ui";

export interface SavedNoteProps {
  /** What went wrong with the last save, in a sentence; nothing when nothing did. */
  readonly error: string | null;
  readonly saved: boolean;
}

/** What the last save did: it failed with a sentence, or it is saved. */
export function SavedNote({ error, saved }: SavedNoteProps) {
  if (error !== null) {
    return <Alert tone={alertTone.danger}>{error}</Alert>;
  }
  return saved ? <Alert tone={alertTone.success}>{t("labels.saved")}</Alert> : null;
}
