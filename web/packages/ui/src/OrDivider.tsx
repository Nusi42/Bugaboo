import styles from "./OrDivider.module.css";

export interface OrDividerProps {
  /** The word between the two ways of doing something. */
  readonly label: string;
}

/** A line with a word in the middle, between two ways of doing the same thing. */
export function OrDivider({ label }: OrDividerProps) {
  return <div className={styles.divider}>{label}</div>;
}
