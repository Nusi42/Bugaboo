import { t } from "@bugaboo/messages";
import { part, partOf } from "@bugaboo/ui";
import type { PointerEvent } from "react";

import { DragHandle } from "../../shared/drag-and-drop.tsx";
import type { Move } from "./dates.ts";
import { dragKinds } from "./drag-kinds.ts";
import type { DragHandlers } from "./use-bar-drag.ts";

/** A press on the dot is the dot's: it does not also start a drag of the bar. */
function keepToTheDot(event: PointerEvent<HTMLElement>): void {
  event.stopPropagation();
}

export interface BarHandlesProps {
  readonly issue: string;
  readonly handlers: (move: Move) => DragHandlers;
}

/**
 * What a selected bar shows for the pointer (UX §4.7): a handle at each end to move one of its
 * dates, and a dot after its end to drag a dependency to another bar. They are hidden from
 * assistive technology: the keys of the bar and *Add dependency…* do the same.
 */
export function BarHandles({ issue, handlers }: BarHandlesProps) {
  return (
    <>
      <span
        {...partOf(part.timelineBarHandle, { variants: ["start"] })}
        {...handlers("start")}
        aria-hidden
      />
      <span
        {...partOf(part.timelineBarHandle, { variants: ["end"] })}
        {...handlers("end")}
        aria-hidden
      />
      <span onPointerDown={keepToTheDot}>
        <DragHandle
          kind={dragKinds.dependency.kind}
          id={issue}
          label={t("timeline.add-dependency")}
          look={partOf(part.timelineBarDot)}
        >
          <span {...partOf(part.timelineBarDotMark)} />
        </DragHandle>
      </span>
    </>
  );
}
