import { partOf, part } from "@bugaboo/ui";
import { useId, type ReactNode } from "react";

import { ItemCount } from "./ItemCount.tsx";

export interface SectionProps {
  readonly heading: string;
  /** The number beside the heading; none for a section that is not a list of things to count. */
  readonly count?: number;
  readonly children: ReactNode;
}

/** A section of *My agents*: a heading with its count, and what it holds. */
export function Section({ heading, count, children }: SectionProps) {
  const id = useId();
  return (
    <section {...partOf(part.stack)} aria-labelledby={id}>
      <h2 id={id} {...partOf(part.agentsHeading)}>
        {heading}
        {count === undefined ? null : <ItemCount count={count} />}
      </h2>
      {children}
    </section>
  );
}
