import { Fragment, type ReactNode } from "react";

import { factSeparator } from "./facts.ts";
import { partOf } from "./parts.ts";
import { part } from "./tokens.ts";

/**
 * A length of time (`1h 30m`), the one way it is drawn: in the code font and never broken between
 * its hours and its minutes (UX §7.3, TIME-03). The words are written by the one writer of lengths.
 */
export function Length({ text }: { readonly text: string }) {
  return <span {...partOf(part.length)}>{text}</span>;
}

/**
 * Facts side by side (*10h estimated · 6h 40m left · 3h 20m spent*): each fact stays on one line,
 * and a line breaks only before the mark that starts the next one, so no mark ends a line.
 */
export function Facts({ items }: { readonly items: readonly Fact[] }) {
  return (
    <span {...partOf(part.factLine)}>
      {items.map((item, at) => (
        <Fragment key={item.key}>
          {at === 0 ? null : " "}
          <span {...partOf(part.factLineItem)}>
            {at === 0 ? null : `${factSeparator} `}
            {item.node}
          </span>
        </Fragment>
      ))}
    </span>
  );
}

/** One fact: what tells it from the others, and what it shows. */
export interface Fact {
  readonly key: string;
  readonly node: ReactNode;
}
