import { Card, partOf, part } from "@bugaboo/ui";
import { useLocation } from "@tanstack/react-router";
import { useMemo, type ReactNode } from "react";
import { createPortal } from "react-dom";

import { CONTENT_ID } from "./content-id.ts";
import { usePageTrail, type TrailStep } from "./page-trail.ts";
import { stepsOfScreens } from "./steps-of-screens.ts";

/** The trail of the screen of an item: where the list is, then the item, so the bar has its name and Back leads to the list. */
function useItemScreenTrail(name: string): void {
  const { pathname } = useLocation();
  const steps = useMemo((): readonly TrailStep[] => {
    const base = stepsOfScreens(pathname);
    const list = base.at(-1);
    return list === undefined ? [] : [...base, { label: name, href: list.href }];
  }, [pathname, name]);
  usePageTrail(steps);
}

export interface ItemScreenProps {
  /** The name of the item: the title in the bar of the screen. */
  readonly name: string;
  readonly children: ReactNode;
}

/**
 * On a phone an item of a list is a screen of its own (like a person): it is drawn in the content of the page, where
 * it replaces everything else, with its name in the top bar and Back to the list. It keeps the context of the page,
 * so what it shows reads and writes as it did in the panel.
 */
export function ItemScreen({ name, children }: ItemScreenProps) {
  useItemScreenTrail(name);
  const place = document.querySelector(`#${CONTENT_ID}`);
  return place === null
    ? null
    : createPortal(
        <section {...partOf(part.itemScreen)}>
          <Card heading={name} level={1}>
            {children}
          </Card>
        </section>,
        place,
      );
}
