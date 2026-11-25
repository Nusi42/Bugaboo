import { t } from "@bugaboo/messages";
import { ActionMenu, Announce, MoreButton, type MenuAction } from "@bugaboo/ui";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";

import { usePinning, type Pin } from "./pins.ts";
import { useMovePin } from "./use-move-pin.ts";

/** What the menu of a pin offers: where the pin can go, and *Unpin*. */
function actionsOf(
  place: number,
  total: number,
  move: (by: number) => void,
  unpin: () => void,
): MenuAction[] {
  const up = {
    id: "up",
    label: t("pins.up"),
    onAction: () => {
      move(-1);
    },
  };
  const down = {
    id: "down",
    label: t("pins.down"),
    onAction: () => {
      move(1);
    },
  };
  return [
    ...(place > 0 ? [up] : []),
    ...(place < total - 1 ? [down] : []),
    { id: "unpin", label: t("pins.unpin"), onAction: unpin },
  ];
}

/** The menu of a pin: *Move up* and *Move down* (the way to move it without a pointer) and *Unpin*. */
export function PinMenu({
  pin,
  place,
  pins,
  onUnpinned,
}: {
  readonly pin: Pin;
  readonly place: number;
  readonly pins: readonly Pin[];
  /** Called when the server has taken the pin away: the section keeps the focus in reach. */
  readonly onUnpinned: () => void;
}) {
  const { move, said } = useMovePin(pin, place, pins);
  const pinning = usePinning({ kind: pin.kind, target: pin.id });
  const [refused, setRefused] = useState<string | null>(null);
  // A refused *Unpin* is said, and the focus is moved only for one that went through.
  const unpin = (): void => {
    pinning.turn({
      onSuccess: onUnpinned,
      onError: (error: unknown) => {
        setRefused(messageOf(error));
      },
    });
  };
  return (
    <>
      <ActionMenu
        label={t("pins.menu", { name: pin.name })}
        actions={actionsOf(place, pins.length, move, unpin)}
        cancelLabel={t("common.cancel")}
        trigger={MoreButton}
      />
      {(said ?? refused) === null ? null : <Announce>{said ?? refused}</Announce>}
    </>
  );
}
