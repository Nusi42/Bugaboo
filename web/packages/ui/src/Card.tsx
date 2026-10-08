import { useId, type ReactNode } from "react";
import { partOf } from "./parts.ts";
import { part } from "./tokens.ts";

export interface CardProps {
  /** The name of the card, shown as its heading and used as its name for screen readers. */
  readonly heading: string;
  /** The level of the heading: 1 for the one card that is the whole page, 2 inside a page. */
  readonly level?: 1 | 2;
  readonly children: ReactNode;
}

/** A region of a page with a heading: a form, a list, a group of settings. */
export function Card({ heading, level = 2, children }: CardProps) {
  const headingId = useId();
  const Heading = level === 1 ? "h1" : "h2";
  return (
    <section {...partOf(part.card)} aria-labelledby={headingId}>
      <Heading id={headingId} {...partOf(part.cardHeading)} tabIndex={-1}>
        {heading}
      </Heading>
      <div {...partOf(part.stack)}>{children}</div>
    </section>
  );
}
