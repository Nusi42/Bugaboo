import { useId, type ReactNode } from "react";
import { partOf } from "./parts.ts";
import type { CardLook } from "./props.ts";
import { part } from "./tokens.ts";

export interface CardProps {
  /** The name of the card, shown as its heading and used as its name for screen readers. */
  readonly heading: string;
  /** The level of the heading: 1 for the one card that is the whole page, 2 inside a page. */
  readonly level?: 1 | 2;
  /** How the card looks besides the usual. */
  readonly look?: CardLook;
  /** What can be done with the whole card, at the end of its heading row: *Customize*. */
  readonly actions?: ReactNode;
  readonly children: ReactNode;
}

/** A region of a page with a heading: a form, a list, a group of settings. */
export function Card({ heading, level = 2, look, actions, children }: CardProps) {
  const headingId = useId();
  const Heading = level === 1 ? "h1" : "h2";
  return (
    <section {...partOf(part.card, { variants: [look] })} aria-labelledby={headingId}>
      <div {...partOf(part.cardHeader)}>
        <Heading id={headingId} {...partOf(part.cardHeading)} tabIndex={-1}>
          {heading}
        </Heading>
        {actions === undefined ? null : <div {...partOf(part.cardActions)}>{actions}</div>}
      </div>
      <div {...partOf(part.stack)}>{children}</div>
    </section>
  );
}
