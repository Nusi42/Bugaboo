import { t } from "@bugaboo/messages";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { useOrderPins, type Pin } from "./pins.ts";

/**
 * The way to move a pin up or down: the whole order goes to the server, and what the person is told (its new place, or
 * why it was refused) is told once the server has answered.
 */
export function useMovePin(pin: Pin, place: number, pins: readonly Pin[]) {
  const order = useOrderPins();
  const [said, setSaid] = useState<string | null>(null);
  const move = (by: number): void => {
    const moved = pins.map(({ kind, id }) => ({ kind, target: id }));
    const [taken] = moved.splice(place, 1);
    if (taken === undefined) {
      return;
    }
    moved.splice(place + by, 0, taken);
    order.mutate(
      { body: { pins: moved } },
      {
        onSuccess: () => {
          setSaid(t("pins.moved", { name: pin.name, place: place + by + 1, total: pins.length }));
        },
        onError: (error: unknown) => {
          setSaid(messageOf(error));
        },
      },
    );
  };
  return { move, said };
}
