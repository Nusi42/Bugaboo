import { t } from "@bugaboo/messages";
import { Button, buttonLook, partOf, part } from "@bugaboo/ui";

import type { IssueSelection } from "./use-selection.ts";

/** How many issues are ticked, with the way to clear them; nothing while none is. */
export function Selected({ selection }: { readonly selection: IssueSelection }) {
  const count = selection.ids.length;
  if (count === 0) {
    return null;
  }
  return (
    <span {...partOf(part.issueNote)}>
      <span>{t("issues.selected", { count })}</span>
      <Button look={buttonLook.quiet} onPress={selection.clear}>
        {t("issues.clear-selection")}
      </Button>
    </span>
  );
}
