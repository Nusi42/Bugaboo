import { Card, partOf, part } from "@bugaboo/ui";
import { useLocation } from "@tanstack/react-router";
import { useEffect, useMemo, useSyncExternalStore, type ReactNode } from "react";
import { createPortal } from "react-dom";

import { doNothing } from "../../shared/do-nothing.ts";
import { CONTENT_ID } from "./content-id.ts";
import { usePageTrail, type TrailStep } from "./page-trail.ts";
import { useEscape } from "./use-escape.ts";
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
  /** Closes the screen: Esc does it, as it does for the panel beside the list on a computer. */
  readonly onClose: () => void;
  readonly children: ReactNode;
}

/** Nothing tells this when the page is drawn; the content is read once more after the first drawing. */
function noChange(): () => void {
  return doNothing;
}

/** The screen itself: Esc closes it, its heading takes the focus when it opens, and the focus goes back when it closes. */
function Screen({ name, onClose, children }: ItemScreenProps) {
  const screen = useEscape<HTMLElement>(onClose);
  useEffect(() => {
    const before = document.activeElement;
    screen.current?.querySelector("h1")?.focus();
    return () => {
      requestAnimationFrame(() => {
        if (before instanceof HTMLElement && before.isConnected) {
          before.focus();
        }
      });
    };
  }, [screen]);
  return (
    <section ref={screen} {...partOf(part.itemScreen)}>
      <Card heading={name} level={1}>
        {children}
      </Card>
    </section>
  );
}

/**
 * On a phone an item of a list is a screen of its own (like a person): it is drawn in the content of the page, where
 * it replaces everything else, with its name in the top bar and Back to the list. It keeps the context of the page,
 * so what it shows reads and writes as it did in the panel.
 */
export function ItemScreen(props: ItemScreenProps) {
  useItemScreenTrail(props.name);
  // The content of the page is there once the page is drawn: it is read again after the first drawing, and the screen is put in it.
  const place = useSyncExternalStore(
    noChange,
    () => document.querySelector(`#${CONTENT_ID}`),
    () => null,
  );
  return place === null ? null : createPortal(<Screen {...props} />, place);
}
