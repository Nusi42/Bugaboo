import { t } from "@bugaboo/messages";
import { SwipeActions, useLayout, useSwipe, type RowActions, type SwipeSide } from "@bugaboo/ui";
import type { ReactNode } from "react";

/** What a row of a list needs to be swiped on a phone (UX §3.3). */
export interface RowSwipe {
  /** The handlers of the finger for the row's own element; none where the layout is not a phone's. */
  readonly rowProps: Partial<ReturnType<typeof useSwipe>["rowProps"]>;
  /** The row was touched or focused: what an action needs to read (whether the person follows) is read from now on. */
  readonly awake: boolean;
  /** The side a swipe opened and the row's *More*, named by `what` (*More: WEB-12*); nothing outside a phone. */
  readonly actions: (actions: RowActions, what: string) => ReactNode;
}

/**
 * The swipe of a row of a list: only a phone swipes, and the row's *More* holds the same actions, so nothing depends
 * on a swipe (principle 9). One hook for every list that swipes: the issue list, notifications and *My agents*.
 */
export function useRowSwipe(sides: readonly SwipeSide[]): RowSwipe {
  const swipe = useSwipe(sides);
  const phone = useLayout() === "phone";
  return {
    rowProps: phone ? swipe.rowProps : {},
    awake: swipe.awake,
    actions: (actions, what) =>
      phone ? (
        <SwipeActions
          actions={actions}
          swipe={swipe}
          moreLabel={t("common.row-more", { what })}
          cancelLabel={t("common.cancel")}
        />
      ) : null,
  };
}
