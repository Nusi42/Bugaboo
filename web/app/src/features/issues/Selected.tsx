import { t } from "@bugaboo/messages";
import { Announce, Button, buttonLook, partOf, part, useHeightOnParent } from "@bugaboo/ui";
import { useRef } from "react";

import type { FoundSearch } from "./found-issues.ts";
import { SelectionBars } from "./SelectionBars.tsx";
import { TickAll } from "./TickAll.tsx";
import type { IssueSelection } from "./use-selection.ts";

export interface SelectedProps {
  readonly selection: IssueSelection;
  /** The project whose issues are listed: its short name. */
  readonly project: string;
  /** What the list is looking for, so *Tick all results* can read every result. */
  readonly search: FoundSearch;
  /** The issues the list draws now. */
  readonly shown: readonly string[];
}

/**
 * The bar of the ticked issues (UX §4.2), over the foot of the list, on a phone too: it slides up
 * over the last rows, nothing in the list moves, and the list keeps room at its end, the height of
 * the bar (UX §6.4). It holds how many are ticked, the way to clear them, what ticks all the
 * results, and the bars that act on them (changing many issues at once, WP-08; the bars decide for
 * themselves when to show).
 */
export function Selected({ selection, project, search, shown }: SelectedProps) {
  const count = selection.ids.length;
  const section = useRef<HTMLElement>(null);
  useHeightOnParent(section, "bar-ticked-height");
  const bar = (
    <section
      ref={section}
      {...partOf(part.selectedBar, { states: [count > 0 && "active"] })}
      aria-label={count > 0 ? t("issues.selected-bar") : undefined}
    >
      {count === 0 ? null : (
        <div {...partOf(part.row)}>
          <span aria-hidden>{t("issues.selected", { count })}</span>
          <Button look={buttonLook.quiet} onPress={selection.clear}>
            {t("issues.clear-selection")}
          </Button>
        </div>
      )}
      <TickAll search={search} shown={shown} selection={selection} />
      <SelectionBars selection={selection} project={project} />
    </section>
  );
  // The count is told as it changes (UX §8); the visible words are left out of the reading. It stands beside the bar, so
  // a bar with nothing to show is empty and takes no room.
  const told = <Announce>{count === 0 ? "" : t("issues.selected", { count })}</Announce>;
  return (
    <>
      {told}
      {bar}
    </>
  );
}
