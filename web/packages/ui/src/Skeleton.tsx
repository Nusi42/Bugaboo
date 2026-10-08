import { partOf } from "./parts.ts";
import { part } from "./tokens.ts";

export interface SkeletonProps {
  /** How many grey lines stand in for the coming text. */
  readonly lines?: number;
}

/**
 * Grey shapes where content is coming (UX §6.1). They only decorate; the part of the page that is
 * loading says so with `aria-busy`. Show them through `useAfterDelay`, so nothing flickers when the
 * answer is quick.
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
