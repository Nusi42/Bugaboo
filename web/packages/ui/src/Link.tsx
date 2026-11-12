import type { ReactNode } from "react";
import { Link as AriaLink } from "react-aria-components";

import { ariaCurrent } from "./aria-current.ts";
import { partOf } from "./parts.ts";
import { part } from "./tokens.ts";

export interface LinkProps {
  /** Where the link goes. An address of the app does not reload the page. */
  readonly href: string;
  readonly children: ReactNode;
  /** The link to the page that is open now, as in a menu. */
  readonly current?: boolean;
  /** The name of a row that opens a page: plain words, and the whole row is the link. */
  readonly row?: boolean;
  /** The address is outside the app (a branch, a pull request): it opens in a new tab and tells that page nothing of this one. */
  readonly external?: boolean;
  /** The link stands among outlined buttons and has their look (*Change* in a row of a list). */
  readonly button?: boolean;
  /** The name for screen readers, where the words alone do not say what the link is about. */
  readonly label?: string;
}

/** A link. With a `RouterProvider` above it, links to pages of the app do not reload the page. */
export function Link({
  href,
  children,
  current = false,
  row = false,
  external = false,
  button = false,
  label,
}: LinkProps) {
  return (
    <AriaLink
      href={href}
      {...partOf(part.link, { variants: [row && "row", button && "button"] })}
      {...(label === undefined ? {} : { "aria-label": label })}
      {...(current ? { "aria-current": ariaCurrent.page } : {})}
      {...(external ? { target: "_blank", rel: "noopener noreferrer" } : {})}
    >
      {children}
    </AriaLink>
  );
}
