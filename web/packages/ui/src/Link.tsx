import type { ReactNode } from "react";
import { Link as AriaLink } from "react-aria-components";

import { partOf } from "./parts.ts";
import { part } from "./tokens.ts";

/** The value of `aria-current` for the page that is open: a word of the ARIA standard. */
const currentPage = "page";

export interface LinkProps {
  /** Where the link goes. An address of the app does not reload the page. */
  readonly href: string;
  readonly children: ReactNode;
  /** The link to the page that is open now, as in a menu. */
  readonly current?: boolean;
  /** The name of a row that opens a page: plain words, and the whole row is the link. */
  readonly row?: boolean;
}

/** A link. With a `RouterProvider` above it, links to pages of the app do not reload the page. */
export function Link({ href, children, current = false, row = false }: LinkProps) {
  return (
    <AriaLink
      href={href}
      {...partOf(part.link, { variants: [row && "row"] })}
      {...(current ? { "aria-current": currentPage } : {})}
    >
      {children}
    </AriaLink>
  );
}
