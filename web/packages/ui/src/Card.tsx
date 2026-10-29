import { useId, type ReactNode } from "react";
import { Link } from "./Link.tsx";
import { useLayout } from "./layout.ts";
import { partOf } from "./parts.ts";
import type { CardLook } from "./props.ts";
import { useTitleInBar } from "./title-bar.ts";
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
  /** The link at the foot that goes back up (*All people*): not drawn on a phone, where the bar of the screen has Back. */
  readonly back?: { readonly href: string; readonly label: string };
  readonly children?: ReactNode;
}

/** The link at the foot of a page that goes back up, on a row of its own. */
function BackLink({ href, label }: { readonly href: string; readonly label: string }) {
  return (
    <div {...partOf(part.row)}>
      <Link href={href}>{label}</Link>
    </div>
  );
}

/** A region of a page with a heading: a form, a list, a group of settings. */
export function Card({ heading, level = 2, look, actions, back, children }: CardProps) {
  const headingId = useId();
  const phone = useLayout() === "phone";
  const titleInBar = useTitleInBar();
  // The one card that is the whole page (level 1) has its heading for screen readers only on a phone where a top bar
  // carries the title, and a screen has one title (UX §7.3); a page without a top bar (the sign-in) shows its heading.
  const hidden = level === 1 && phone && titleInBar;
  const Heading = level === 1 ? "h1" : "h2";
  // A card with nothing to show is not drawn: where its heading is for screen readers only and nothing is in it, only the heading stays.
  const backLink = back === undefined || phone ? null : <BackLink {...back} />;
  const nothing =
    (children === null || children === undefined || children === false) && backLink === null;
  if (hidden && nothing && actions === undefined) {
    return (
      <Heading id={headingId} {...partOf(part.visuallyHidden)} tabIndex={-1}>
        {heading}
      </Heading>
    );
  }
  // The heading that is for screen readers only takes no room: there is no slot for it above the first line.
  const bare = hidden && actions === undefined;
  return (
    <section {...partOf(part.card, { variants: [look] })} aria-labelledby={headingId}>
      {bare ? (
        <Heading id={headingId} {...partOf(part.visuallyHidden)} tabIndex={-1}>
          {heading}
        </Heading>
      ) : (
        <div {...partOf(part.cardHeader)}>
          <Heading
            id={headingId}
            {...partOf(hidden ? part.visuallyHidden : part.cardHeading)}
            tabIndex={-1}
          >
            {heading}
          </Heading>
          {actions === undefined ? null : <div {...partOf(part.cardActions)}>{actions}</div>}
        </div>
      )}
      <div {...partOf(part.stack)}>
        {children}
        {backLink}
      </div>
    </section>
  );
}
