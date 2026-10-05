import type { ReactNode } from "react";
import { Link as AriaLink } from "react-aria-components";

import { cx } from "./cx.ts";
import styles from "./Link.module.css";

/** The value of `aria-current` for the page that is open: a word of the ARIA standard. */
const currentPage = "page";

export interface LinkProps {
  /** Where the link goes. An address of the app does not reload the page. */
  readonly href: string;
  readonly children: ReactNode;
  /** The link to the page that is open now, as in a menu. */
  readonly current?: boolean;
}

/** A link. With a `RouterProvider` above it, links to pages of the app do not reload the page. */
export function Link({ href, children, current = false }: LinkProps) {
  return (
    <AriaLink
      href={href}
      className={cx(styles.link)}
      {...(current ? { "aria-current": currentPage } : {})}
    >
      {children}
    </AriaLink>
  );
}
