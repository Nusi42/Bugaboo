import { far, LEAST_MOVE_PX } from "./movement-frames.ts";
import type { Box } from "./movement-record.ts";

// The rule of the first read (UX §6.4, M8 WP-13), pure so it is tested without a browser
// (`first-read-judge.test.ts`): once a part is drawn, it does not move. Grey shapes may go and the content be drawn
// where they stood, and a part may grow at its end; nothing else of it changes.

/** Where a part stood from a frame on: `null` while it was not drawn. */
export interface Sighting {
  readonly frame: number;
  readonly box: Box | null;
}

/** A part of the screen from the first frame of the page: its name, and each change of its box. */
export interface Track {
  readonly name: string;
  readonly boxes: readonly Sighting[];
}

/** The size of the window: a part outside it is not seen. */
export interface View {
  readonly width: number;
  readonly height: number;
}

/** A part that moved after it was drawn: where it was first drawn, where it went, and in which frame. */
export interface FirstMove {
  readonly name: string;
  readonly from: Box;
  readonly to: Box;
  readonly frame: number;
}

/** Whether some of the box shows in the window. */
function seen(box: Box, view: View): boolean {
  return (
    box.y < view.height && box.y + box.height > 0 && box.x < view.width && box.x + box.width > 0
  );
}

/** Whether the part stands where it was first drawn, as big or bigger: grown at its end. */
function inPlace(first: Box, later: Box): boolean {
  return (
    !far(first.x, later.x) &&
    !far(first.y, later.y) &&
    later.width >= first.width - LEAST_MOVE_PX &&
    later.height >= first.height - LEAST_MOVE_PX
  );
}

/** The first move of a part after it was drawn in view, if it made one. */
function firstMoveOf(track: Track, view: View): FirstMove | undefined {
  const drawn = track.boxes.filter((each) => each.box !== null);
  const start = drawn.findIndex((each) => each.box !== null && seen(each.box, view));
  const first = drawn[start]?.box;
  if (first === undefined || first === null) {
    return undefined;
  }
  const moved = drawn
    .slice(start + 1)
    .find((each) => each.box !== null && !inPlace(first, each.box));
  return moved === undefined || moved.box === null
    ? undefined
    : { name: track.name, from: first, to: moved.box, frame: moved.frame };
}

/** Every part that moved after it was first drawn in view (UX §6.4): none, when the first read held the rule. */
export function movedOnFirstRead(tracks: readonly Track[], view: View): readonly FirstMove[] {
  return tracks.flatMap((track) => {
    const move = firstMoveOf(track, view);
    return move === undefined ? [] : [move];
  });
}
