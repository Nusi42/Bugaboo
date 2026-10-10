import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, partOf, part } from "@bugaboo/ui";

import type { IssueConflict } from "./use-issue-changes.ts";

/**
 * Somebody changed a short value first (UX §4.4, M3-Q5 d): the sentence built from the structured
 * answer of the server, and the two answers. The change of the person was taken back from the
 * screen; *Use mine* sends it again on the newest version.
 */
export function ConflictNote({ conflict }: { readonly conflict: IssueConflict }) {
  return (
    <Alert tone={alertTone.warning}>
      <div {...partOf(part.issueNote)}>
        <span>{conflict.sentence}</span>
        <Button onPress={conflict.keepTheirs}>
          {t("issues.keep-theirs", { name: conflict.who })}
        </Button>
        <Button onPress={conflict.useMine}>{t("issues.use-mine")}</Button>
      </div>
    </Alert>
  );
}
