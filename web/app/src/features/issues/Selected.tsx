import { t } from "@bugaboo/messages";
import { Announce, Button, buttonLook, partOf, part, useLayout } from "@bugaboo/ui";
import { use } from "react";
import { createPortal } from "react-dom";

import { BarSlot } from "../shell/index.ts";

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
 * The bar of the ticked issues (UX §4.2), under the list so that ticking moves nothing above it, and
 * stuck to the bottom of the window while the list is longer than it (on a phone it stands in the
 * column of the app above the bottom navigation, with the same content): how many are ticked, the way to
 * clear them, what ticks all the results, and the bars that act on them (changing many issues at
 * once, WP-08; the bars decide for themselves when to show).
 */
export function Selected({ selection, project, search, shown }: SelectedProps) {
  const count = selection.ids.length;
  // A phone has the bar where the bar of an issue is: under the page and over nothing.
  const slot = use(BarSlot);
  const phone = useLayout() === "phone";
  const bar = (
    <section
      {...partOf(part.selectedBar, { states: [count > 0 && "active"] })}
      aria-label={count > 0 ? t("issues.selected-bar") : undefined}
    >
      {/* The count is told as it changes (UX §8); the visible words are left out of the reading. */}
      <Announce>{count === 0 ? "" : t("issues.selected", { count })}</Announce>
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
  return phone && slot !== null ? createPortal(bar, slot) : bar;
}
