import { t } from "@bugaboo/messages";
import { useState } from "react";

import { removeKeepingFocus } from "../../shared/focus.ts";
import { undoStore } from "../../shared/optimistic-undo.ts";

/** One of the example items of the frames page. */
export interface Example {
  readonly id: string;
  readonly name: string;
}

const examples: readonly Example[] = [
  { id: "first", name: t("frames.item-1") },
  { id: "second", name: t("frames.item-2") },
  { id: "third", name: t("frames.item-3") },
];

/**
 * The example items that are still there. Removing one is the example of a reversible action
 * (UX §6): it goes at once and *Undo* is on offer for 10 seconds.
 */
export function useExamples(): {
  readonly items: readonly Example[];
  readonly remove: (example: Example) => void;
} {
  const [removed, setRemoved] = useState<readonly string[]>([]);
  return {
    items: examples.filter((example) => !removed.includes(example.id)),
    remove: (example) => {
      removeKeepingFocus(() => {
        setRemoved((now) => [...now, example.id]);
      });
      undoStore.offer({
        message: t("frames.removed", { name: example.name }),
        undo: () => {
          setRemoved((now) => now.filter((id) => id !== example.id));
        },
        restoredFocus: () =>
          document.querySelector<HTMLElement>(
            `[aria-label="${CSS.escape(t("frames.remove-name", { name: example.name }))}"]`,
          ),
      });
    },
  };
}
