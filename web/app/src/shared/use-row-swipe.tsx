import { t } from "@bugaboo/messages";
import {
  SwipeActions,
  useLayout,
  useSwipe,
  type RowActions,
  type Swipe,
  type SwipeSide,
} from "@bugaboo/ui";
import type { ReactNode } from "react";

import { listText } from "./list-text.ts";

/** What a row of a list needs to be swiped on a phone (UX §3.3). */
export interface RowSwipe {
  /** The handlers of the finger for the row's own element; none where the layout is not a phone's. */
  readonly rowProps: Partial<Swipe["rowProps"]>;
  readonly swipe: Swipe;
  readonly phone: boolean;
}

/**
 * The swipe of a row of a list: only a phone swipes, over the `sides` that have actions. `onTouch` hears that the row
 * was touched.
 */
export function useRowSwipe(sides: readonly SwipeSide[], onTouch?: () => void): RowSwipe {
  const swipe = useSwipe(sides, onTouch);
  const phone = useLayout() === "phone";
  return { rowProps: phone ? swipe.rowProps : {}, swipe, phone };
}

/**
 * The side a swipe opened and the row's *More*, which holds the same actions, so nothing depends on a swipe
 * (principle 9); `what` names the row in the name of *More* (*More: WEB-12*). Nothing outside a phone.
 */
export function swipeMore(row: RowSwipe, actions: RowActions, what: string): ReactNode {
  return row.phone ? (
    <SwipeActions
      actions={actions}
      swipe={row.swipe}
      moreLabel={t("common.row-more", { what })}
      cancelLabel={t("common.cancel")}
      announce={(names) => t("common.swipe-shown", { actions: listText(names) })}
    />
  ) : null;
}
