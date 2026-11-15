import { useAfterDelay } from "./after-delay.ts";
import { partOf } from "./parts.ts";
import { part } from "./tokens.ts";

export interface SkeletonProps {
  /** How many grey lines stand in for the coming text. */
  readonly lines?: number;
}

/**
 * Grey shapes where content is coming (UX §6.1). They only decorate; the part of the page that is
 * loading says so with `aria-busy`. Show them through `DelayedSkeleton` (or `useAfterDelay`), so
 * nothing flickers when the answer is quick.
 */
/** The last line is the shortest, and every second one is a little shorter. */
function shapeOf(index: number, lines: number): "short" | "medium" | undefined {
  if (index === lines - 1) {
    return "short";
  }
  return index % 2 === 1 ? "medium" : undefined;
}

export function Skeleton({ lines = 3 }: SkeletonProps) {
  return (
    <div {...partOf(part.column, { variants: ["snug"] })} aria-hidden>
      {Array.from({ length: lines }, (_, index) => (
        <span key={index} {...partOf(part.skeletonLine, { variants: [shapeOf(index, lines)] })} />
      ))}
    </div>
  );
}

/**
 * Grey shapes for content that is read for the first time: nothing for the first 0.3 s, then the
 * shapes (UX §6.1), so a quick answer never flashes them. A part that waits draws this, not
 * `Skeleton` itself; a part that is read again keeps what it shows (UX §6).
 */
export function DelayedSkeleton({ lines = 3 }: SkeletonProps) {
  return useAfterDelay(true) ? <Skeleton lines={lines} /> : null;
}
