import { partOf } from "./parts.ts";
import { part } from "./tokens.ts";

export interface OrDividerProps {
  /** The word between the two ways of doing something. */
  readonly label: string;
}

/** A line with a word in the middle, between two ways of doing the same thing. */
export function OrDivider({ label }: OrDividerProps) {
  return <div {...partOf(part.orDivider)}>{label}</div>;
}
