import { t } from "@bugaboo/messages";
import { Button, buttonLook, partOf, part } from "@bugaboo/ui";

import { SelectionBars } from "./SelectionBars.tsx";
import type { IssueSelection } from "./use-selection.ts";

/**
 * How many issues are ticked, with the way to clear them, and the bars that act on them (changing
 * many issues at once, WP-08); nothing while none is.
 */
export function Selected({ selection }: { readonly selection: IssueSelection }) {
  const count = selection.ids.length;
  if (count === 0) {
    return null;
  }
  return (
    <div {...partOf(part.stack)}>
      <span {...partOf(part.issueNote)}>
        <span>{t("issues.selected", { count })}</span>
        <Button look={buttonLook.quiet} onPress={selection.clear}>
          {t("issues.clear-selection")}
        </Button>
      </span>
      <SelectionBars selection={selection} />
    </div>
  );
}
