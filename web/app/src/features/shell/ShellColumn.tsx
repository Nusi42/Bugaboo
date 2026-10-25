import { type LayoutMode, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { usePageFocus } from "../../shared/focus.ts";
import { BarSlot } from "./bar-slot.ts";
import { CONTENT_ID, scrollRestoration } from "./content-id.ts";
import { TopBar } from "./TopBar.tsx";
import { PageColumn } from "./PageColumn.tsx";
import { PhoneBars } from "./PhoneBars.tsx";

export interface ShellColumnProps {
  readonly layout: LayoutMode;
  /** Why signing out did not work, if it did not. */
  readonly signOutProblem: unknown;
}

/** Everything next to the sidebar: the top bar, the page, and on a phone the bar at the bottom. */
export function ShellColumn({ layout, signOutProblem }: ShellColumnProps) {
  usePageFocus(CONTENT_ID);
  const phone = layout === "phone";
  const [slot, setSlot] = useState<HTMLElement | null>(null);
  return (
    <BarSlot value={slot}>
      <div {...partOf(part.appLayoutColumn)}>
        <TopBar layout={layout} />
        <main id={CONTENT_ID} {...partOf(part.appLayoutMain)} tabIndex={-1} {...scrollRestoration}>
          <PageColumn phone={phone} signOutProblem={signOutProblem} />
        </main>
        {phone ? <PhoneBars onSlot={setSlot} /> : null}
      </div>
    </BarSlot>
  );
}
