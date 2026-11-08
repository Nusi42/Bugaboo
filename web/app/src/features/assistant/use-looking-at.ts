import { useLocation } from "@tanstack/react-router";
import { useState } from "react";

import { lookingAtOf, withoutItem, type LookingAtItem } from "./looking-at.ts";

/** What the panel looks at: from the place it opened at, and the way to leave an item out. */
export function useLookingAt(): {
  readonly items: readonly LookingAtItem[];
  readonly leaveOut: (key: string) => void;
} {
  const location = useLocation();
  const [items, setItems] = useState(() => lookingAtOf(location.pathname, location.search));
  return {
    items,
    leaveOut: (key) => {
      setItems((before) => withoutItem(before, key));
    },
  };
}
