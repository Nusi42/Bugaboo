import styles from "./Skeleton.module.css";

export interface SkeletonProps {
  /** How many grey lines stand in for the coming text. */
  readonly lines?: number;
}

/**
 * Grey shapes where content is coming (UX §6.1). They only decorate; the part of the page that is
 * loading says so with `aria-busy`. Show them through `useAfterDelay`, so nothing flickers when the
 * answer is quick.
 */
export function Skeleton({ lines = 3 }: SkeletonProps) {
  return (
    <div className={styles.skeleton} aria-hidden>
      {Array.from({ length: lines }, (_, index) => (
        <span key={index} className={styles.line} />
      ))}
    </div>
  );
}
