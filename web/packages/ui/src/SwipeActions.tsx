import { ActionMenu, type ActionMenuTriggerProps } from "./ActionMenu.tsx";
import { Button } from "./Button.tsx";
import { doNothing } from "./do-nothing.ts";
import { IconButton } from "./IconButton.tsx";
import { iconName } from "./icons.ts";
import { partOf } from "./parts.ts";
import { StatusIcon } from "./StatusIcon.tsx";
import { swipeSide, type SwipeSide } from "./swipe.ts";
import { part } from "./tokens.ts";
import type { RowActions, SideActions, Swipe } from "./use-swipe.ts";

/** The actions of the open side, over the row at its edge; pressing one does it and closes the side. */
function OpenSide({
  side,
  actions,
  close,
}: {
  readonly side: Exclude<SwipeSide, "none">;
  readonly actions: SideActions;
  readonly close: () => void;
}) {
  return (
    <div {...partOf(part.swipeActions, { variants: [side] })}>
      {actions.map((action) => (
        <Button
          key={action.id}
          onPress={() => {
            close();
            action.onAction();
          }}
        >
          {action.group === undefined ? null : <StatusIcon group={action.group} />}
          {action.label}
        </Button>
      ))}
    </div>
  );
}

/** The square button of *More* with its icon; it is named by the row's words. */
function MoreButton({ label, onOpen }: ActionMenuTriggerProps) {
  return <IconButton icon={iconName.more} label={label} onPress={onOpen ?? doNothing} />;
}

export interface SwipeActionsProps {
  readonly actions: RowActions;
  readonly swipe: Swipe;
  /** The name of the row's *More*, with what the row is: *More: WEB-12*. */
  readonly moreLabel: string;
  /** The word of *Cancel* on the sheet of *More*. */
  readonly cancelLabel: string;
}

/**
 * The swipe actions of a row of a list (UX §3.3, UX-02): the side a swipe opened, and the row's *More*, which holds
 * every action of both sides, so nothing depends on a swipe (principle 9, TEST-A11Y). The row spreads
 * `swipe.rowProps` on its own element; *More* stands where the row puts this component.
 */
export function SwipeActions({ actions, swipe, moreLabel, cancelLabel }: SwipeActionsProps) {
  const every = [...(actions.start ?? []), ...(actions.end ?? [])];
  const open = swipe.side === swipeSide.none ? undefined : actions[swipe.side];
  return (
    <>
      {open === undefined || swipe.side === swipeSide.none ? null : (
        <OpenSide side={swipe.side} actions={open} close={swipe.close} />
      )}
      {every.length === 0 ? null : (
        <span {...partOf(part.rowMore)}>
          <ActionMenu
            label={moreLabel}
            actions={every}
            cancelLabel={cancelLabel}
            trigger={MoreButton}
          />
        </span>
      )}
    </>
  );
}
