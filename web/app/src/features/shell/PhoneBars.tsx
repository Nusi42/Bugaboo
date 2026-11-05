import { partOf, part } from "@bugaboo/ui";

import { useAppBarHidden } from "./app-bar.ts";
import { BottomBar } from "./BottomBar.tsx";
import { UndoHost } from "./UndoHost.tsx";

/**
 * What a phone shows under the page, from the top: the offers to undo, the bar of an open issue
 * (which the issue puts into the slot), and the bar of the app, which an open issue hides. All three are parts of the column,
 * so none of them covers the page or another one.
 */
export function PhoneBars({ onSlot }: { readonly onSlot: (slot: HTMLElement | null) => void }) {
  const hidden = useAppBarHidden();
  return (
    <>
      <UndoHost inFlow />
      <div ref={onSlot} {...partOf(part.appLayoutBarSlot)} />
      {hidden ? null : <BottomBar />}
    </>
  );
}
