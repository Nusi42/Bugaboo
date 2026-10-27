import { useEffect, useRef, type RefObject } from "react";

import { firstEnabled, type ResultGroup } from "./quick-search-collect.ts";

/**
 * The name of the event with which an autocomplete box asks its list to move the virtual focus
 * (`react-aria`'s own, the one it sends when the person types).
 */
const FOCUS_EVENT = "react-aria-focus";

/**
 * The first result has the focus of the list (the box names it with `aria-activedescendant`, and
 * Enter opens it) also when a group that comes later puts another result on top, until the person
 * moves the focus. Whether they did is read from where the focus is: it is not on the result that
 * was first when it was last set. The words typed give the focus to the first result again.
 *
 * `root` holds the box and the list; the list moves the focus when it gets the event that the
 * autocomplete box sends it.
 */
export function useFollowFirst(
  root: Readonly<RefObject<HTMLElement | null>>,
  groups: readonly ResultGroup[],
  query: string,
): void {
  const followed = useRef<string | null>(null);
  const asked = useRef(query);
  useEffect(() => {
    if (asked.current !== query) {
      asked.current = query;
      followed.current = null;
    }
    const input = root.current?.querySelector("input");
    const list = root.current?.querySelector("[role='listbox']");
    const first = firstEnabled(groups);
    if (!input || !list || first === undefined) {
      return;
    }
    const id = input.getAttribute("aria-activedescendant");
    const active =
      id === null ? undefined : list.querySelector<HTMLElement>(`[id='${id}']`)?.dataset["key"];
    const moved = active !== undefined && followed.current !== null && active !== followed.current;
    if (moved) {
      return;
    }
    followed.current = first;
    if (active !== first) {
      list.dispatchEvent(
        new CustomEvent(FOCUS_EVENT, {
          bubbles: true,
          cancelable: true,
          detail: { focusStrategy: "first" },
        }),
      );
    }
  }, [root, groups, query]);
}
