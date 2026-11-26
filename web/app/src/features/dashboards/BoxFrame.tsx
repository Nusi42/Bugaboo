import { t } from "@bugaboo/messages";
import {
  buttonLook,
  buttonShape,
  buttonType,
  Icon,
  iconName,
  iconSize,
  part,
  partOf,
  usePlace,
} from "@bugaboo/ui";
import { useKeyboardScroll } from "@bugaboo/ui/scroll-reach";
import {
  useEffect,
  useEffectEvent,
  useRef,
  type KeyboardEvent,
  type ReactNode,
  type RefObject,
} from "react";

import type { Box } from "./dashboards.ts";
import { gridPlaceOf } from "./grid-layout.ts";
import type { DragHandlers } from "./use-grid-drag.ts";

/** For how many frames the handle takes the focus back from a menu that closes. */
const focusTries = 20;

/** How a box can be placed now: by a pointer and by its keys on a computer in *Change*, or not at all. */
export interface BoxPlacing {
  /** The title and the corner drag the box (a computer in *Change*). */
  readonly drag?: { readonly move: DragHandlers; readonly resize: DragHandlers } | undefined;
  /** The box is the one being dragged: it stays, quiet, and its shadow shows where it lands. */
  readonly dragging: boolean;
  /** *Move …* or *Resize …* works on this box with the keys. */
  readonly keys: boolean;
  /** The keys of the handle while it works. */
  readonly onKeyDown: (event: KeyboardEvent<HTMLElement>) => void;
  /** The focus left something in the box: whether it went to something else in the box (`focusout`). */
  readonly onBlur: (stillInBox: boolean) => void;
  /** The handle was pressed with the keyboard: *Move …* starts. */
  readonly onPress: () => void;
}

/**
 * Gives the focus to the element while `on`: the menu that chose *Move …* gives the focus back to its button when it has
 * closed, some frames later, and the handle takes it once that has happened, so the person hears where the box is.
 */
function useFocusWhile(element: Readonly<RefObject<HTMLElement | null>>, on: boolean): void {
  useEffect(() => {
    if (!on) {
      return () => {};
    }
    let tries = focusTries;
    let frame = 0;
    const take = (): void => {
      const target = element.current;
      if (target !== null && document.activeElement !== target) {
        target.focus();
      }
      tries -= 1;
      if (tries > 0) {
        frame = requestAnimationFrame(take);
      }
    };
    frame = requestAnimationFrame(take);
    return () => {
      cancelAnimationFrame(frame);
    };
  }, [element, on]);
}

/**
 * Tells `onBlur` each time the focus leaves something in the element (`focusout`) whether it stays in it: Tab past the
 * menu of a box or a click elsewhere ends *Move …* or *Resize …*, while the menu that chose it gives the focus back to its
 * own button in the box, which is not leaving it.
 */
function useFocusLeaving(
  element: Readonly<RefObject<HTMLElement | null>>,
  onBlur: (stillInBox: boolean) => void,
): void {
  const told = useEffectEvent((stillInBox: boolean) => {
    onBlur(stillInBox);
  });
  useEffect(() => {
    const frame = element.current;
    if (frame === null) {
      return () => {};
    }
    const listen = (event: { readonly relatedTarget: unknown }): void => {
      const next = event.relatedTarget;
      told(next instanceof Node && frame.contains(next));
    };
    frame.addEventListener("focusout", listen);
    return () => {
      frame.removeEventListener("focusout", listen);
    };
  }, [element]);
}

/** The head of a box: its title (in *Change* the handle a pointer drags, with the button that takes the keys), its menu. */
function BoxHead({
  box,
  placing,
  menu,
}: {
  readonly box: Box;
  readonly placing: BoxPlacing;
  readonly menu: ReactNode;
}) {
  const keys = useRef<HTMLButtonElement>(null);
  useFocusWhile(keys, placing.keys);
  const { drag } = placing;
  const title = <h2 {...partOf(part.dashboardBoxTitle)}>{box.title}</h2>;
  return (
    <div {...partOf(part.dashboardBoxHead)}>
      {drag === undefined ? (
        title
      ) : (
        <div {...partOf(part.dashboardBoxHandle)} {...drag.move}>
          <button
            ref={keys}
            type={buttonType.button}
            {...partOf(part.button, { variants: [buttonLook.quiet, buttonShape.square] })}
            aria-label={t("dashboards.place", { title: box.title })}
            onKeyDown={placing.onKeyDown}
            onClick={placing.onPress}
          >
            <Icon name={iconName.grip} size={iconSize.small} />
          </button>
          {title}
        </div>
      )}
      {menu}
    </div>
  );
}

/**
 * What a box shows: it scrolls when it holds more than its place, and then takes the focus itself while nothing in it
 * does, so the keyboard scrolls it too (WCAG 2.1.1).
 */
function BoxBody({ children }: { readonly children: ReactNode }) {
  const body = useRef<HTMLDivElement>(null);
  const reach = useKeyboardScroll(body);
  return (
    <div ref={body} {...partOf(part.dashboardBoxBody)} {...(reach ? { tabIndex: 0 } : {})}>
      {children}
    </div>
  );
}

export interface BoxFrameProps {
  readonly box: Box;
  readonly placing: BoxPlacing;
  /** The menu of the box in *Change*. */
  readonly menu: ReactNode;
  readonly children: ReactNode;
}

/**
 * One box on the grid (UX §4.10): its title (in *Change* the handle a pointer drags, with the button that takes the
 * keys of *Move …* and *Resize …*) and its menu, what it shows, and in *Change* its corner. It stands on its place in
 * the grid by the numbers of the grid of the theme (`box-column` …).
 */
export function BoxFrame({ box, placing, menu, children }: BoxFrameProps) {
  const ref = useRef<HTMLLIElement>(null);
  usePlace(ref, gridPlaceOf(box));
  useFocusLeaving(ref, placing.onBlur);
  const { drag } = placing;
  return (
    <li
      ref={ref}
      {...partOf(part.dashboardBox, {
        states: [placing.dragging && "dragging", placing.keys && "keys"],
      })}
    >
      <BoxHead box={box} placing={placing} menu={menu} />
      <BoxBody>{children}</BoxBody>
      {drag === undefined ? null : (
        // The corner is for the pointer; the keyboard has *Resize …* in the menu.
        <span {...partOf(part.dashboardBoxCorner)} {...drag.resize} aria-hidden>
          <Icon name={iconName.resize} size={iconSize.small} />
        </span>
      )}
    </li>
  );
}
