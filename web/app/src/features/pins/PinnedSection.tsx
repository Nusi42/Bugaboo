import { t } from "@bugaboo/messages";
import { iconName, part, partOf, useHoldWhileReading, useShortWindow } from "@bugaboo/ui";
import { useState } from "react";

import { pinsShown, pinsShownLow } from "../../shared/pin-folds.ts";
import { NavButton, SectionFrame, type SidebarSectionView } from "../shell/index.ts";
import { PinRow } from "./PinRow.tsx";
import { pinsHeading, usePins } from "./pins.ts";
import { useFocusAfterUnpin } from "./use-focus-after-unpin.ts";

/**
 * *Pinned* in the sidebar and at the top of *More* (UX §3.1, §3.3, CFG-05): the projects, saved searches and dashboards the person pinned,
 * in their order, each with its icon and its name and, beside it, its menu. With more than eight the sidebar shows eight and
 * *Show all pinned*, and in a window 600 px high one, so it needs no more scrolling than without pins; narrowed to icons it
 * shows the icons, named for screen readers and on pointing, and *Show all pinned* as an icon too. Nothing is shown for a
 * person with no pins in the sidebar, and a pin to something the person no longer sees is not there at all.
 */
export function PinnedSection({ view }: { readonly view: SidebarSectionView }) {
  const { narrow, place } = view;
  const [all, setAll] = useState(false);
  const low = useShortWindow();
  const read = usePins();
  useHoldWhileReading(read.isLoading);
  const items = read.data?.items;
  const pins = items ?? [];
  const { list, unpinning, keepMenu } = useFocusAfterUnpin(items);
  if (pins.length === 0) {
    return null;
  }
  const limit = low ? pinsShownLow : pinsShown;
  const folds = place === "sidebar" && pins.length > limit;
  const shown = folds && !all ? pins.slice(0, limit) : pins;
  return (
    <SectionFrame heading={pinsHeading} view={view}>
      <ul {...partOf(part.sidebarList)} ref={list}>
        {shown.map((pin, at) => (
          <PinRow
            key={pin.id}
            pin={pin}
            place={at}
            pins={pins}
            view={view}
            menuBox={keepMenu(pin.id)}
            onUnpinned={() => {
              unpinning(pins, at);
            }}
          />
        ))}
        {folds ? (
          <li>
            <NavButton
              icon={all ? iconName.up : iconName.down}
              narrow={narrow}
              label={all ? t("pins.show-fewer") : t("pins.show-all")}
              isExpanded={all}
              onPress={() => {
                setAll(!all);
              }}
            />
          </li>
        ) : null}
      </ul>
    </SectionFrame>
  );
}
