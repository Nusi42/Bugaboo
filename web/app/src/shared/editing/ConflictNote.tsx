import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, partOf, part } from "@bugaboo/ui";

import type { ConflictPrompt } from "./conflict-prompt.ts";

/**
 * Somebody changed a value first (UX §4.4, §4.7, M3-Q5 d): the sentence built from the structured
 * answer of the server, and the two answers. The change of the person was taken back from the
 * screen; *Use mine* sends it again on the newest version. The issue page and the timeline ask with
 * it.
 */
export function ConflictNote({ conflict }: { readonly conflict: ConflictPrompt }) {
  return (
    <Alert tone={alertTone.warning}>
      <div {...partOf(part.issueNote)}>
        <span>{conflict.sentence}</span>
        <Button onPress={conflict.keepTheirs}>{conflict.theirs}</Button>
        <Button onPress={conflict.useMine}>{t("issues.use-mine")}</Button>
      </div>
    </Alert>
  );
}
