import { t } from "@bugaboo/messages";
import { Announce, Button, buttonLook, partOf, part } from "@bugaboo/ui";

import { SelectionBars } from "./SelectionBars.tsx";
import type { IssueSelection } from "./use-selection.ts";

export interface SelectedProps {
  readonly selection: IssueSelection;
  /** The project whose issues are listed: its short name. */
  readonly project: string;
}

/**
 * How many issues are ticked, with the way to clear them, and the bars that act on them (changing
 * many issues at once, WP-08); the bars decide for themselves when to show.
 */
export function Selected({ selection, project }: SelectedProps) {
  const count = selection.ids.length;
  return (
    <div {...partOf(part.stack)}>
      {/* The count is told as it changes (UX §8); the visible words are left out of the reading. */}
      <Announce>{count === 0 ? "" : t("issues.selected", { count })}</Announce>
      {count === 0 ? null : (
        <span {...partOf(part.issueNote)}>
          <span aria-hidden>{t("issues.selected", { count })}</span>
          <Button look={buttonLook.quiet} onPress={selection.clear}>
            {t("issues.clear-selection")}
          </Button>
        </span>
      )}
      <SelectionBars selection={selection} project={project} />
    </div>
  );
}
