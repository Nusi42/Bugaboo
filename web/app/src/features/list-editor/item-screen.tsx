import { Card, partOf, part, inPlace } from "@bugaboo/ui";
import { useLocation } from "@tanstack/react-router";
import { useEffect, useSyncExternalStore, type ReactNode } from "react";
import { createPortal } from "react-dom";

import { doNothing } from "../../shared/do-nothing.ts";
import { CONTENT_ID, useEscape, useItemTrail } from "../shell/index.ts";

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
    screen.current?.querySelector("h1")?.focus(inPlace);
    return () => {
      requestAnimationFrame(() => {
        if (before instanceof HTMLElement && before.isConnected) {
          before.focus(inPlace);
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
  useItemTrail(useLocation().pathname, props.name);
  // The content of the page is there once the page is drawn: it is read again after the first drawing, and the screen is put in it.
  const place = useSyncExternalStore(
    noChange,
    () => document.querySelector(`#${CONTENT_ID}`),
    () => null,
  );
  return place === null ? null : createPortal(<Screen {...props} />, place);
}
