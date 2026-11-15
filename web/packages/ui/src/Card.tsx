import { useId, type ReactNode } from "react";
import { Link } from "./Link.tsx";
import { useLayout } from "./layout.ts";
import { InPaneHead, paneHeadSlot, useInPane } from "./pane-head.ts";
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
  /**
   * What the card is for, in a sentence or two under the heading: the one introduction of every settings page and
   * its sections, small and muted, as wide as the card (UX §5, L3).
   */
  readonly intro?: ReactNode;
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

/** Whether a card has nothing of its own to hold. */
function isEmpty(children: ReactNode): boolean {
  return children === null || children === undefined || children === false;
}

interface CardTopProps {
  readonly id: string;
  readonly heading: string;
  readonly level: 1 | 2;
  /** The heading is for screen readers only. */
  readonly hidden: boolean;
  readonly actions: ReactNode;
}

/** The heading of a card, with what can be done with the whole card at the end of its row. */
function CardTop({ id, heading, level, hidden, actions }: CardTopProps) {
  const Heading = level === 1 ? "h1" : "h2";
  // The heading that is for screen readers only takes no room: there is no slot for it above the first line.
  if (hidden && actions === undefined) {
    return (
      <Heading id={id} {...partOf(part.visuallyHidden)} tabIndex={-1}>
        {heading}
      </Heading>
    );
  }
  return (
    <div {...partOf(part.cardHeader)}>
      <Heading id={id} {...partOf(hidden ? part.visuallyHidden : part.cardHeading)} tabIndex={-1}>
        {heading}
      </Heading>
      {actions === undefined ? null : <div {...partOf(part.cardActions)}>{actions}</div>}
    </div>
  );
}

/** A region of a page with a heading: a form, a list, a group of settings. */
export function Card({ heading, level = 2, look, actions, intro, back, children }: CardProps) {
  const headingId = useId();
  const phone = useLayout() === "phone";
  const titleInBar = useTitleInBar();
  // A phone keeps the heading and the introduction in the card: its top bar carries the title.
  const inPane = useInPane() && level === 1 && !phone;
  // The one card that is the whole page (level 1) has its heading for screen readers only on a phone where a top bar
  // carries the title, and a screen has one title (UX §7.3); a page without a top bar (the sign-in) shows its heading.
  const hidden = level === 1 && phone && titleInBar;
  // A card with nothing to show is not drawn: where its heading is for screen readers only and nothing is in it, only the heading stays.
  const backLink = back === undefined || phone ? null : <BackLink {...back} />;
  const top = (
    <CardTop id={headingId} heading={heading} level={level} hidden={hidden} actions={actions} />
  );
  const introLine = intro === undefined ? null : <p {...partOf(part.cardIntro)}>{intro}</p>;
  const empty = isEmpty(children) && backLink === null;
  if (hidden && empty && intro === undefined && actions === undefined) {
    return top;
  }
  // The one card of a page in a pane on a computer or a tablet: its heading and introduction are the head of the pane,
  // on the page above the rest of it, which stands in the box with what follows (UX §6.5); a card that holds nothing
  // more is not drawn. Both ways are one tree, so what is typed in the card stays when the layout changes.
  return (
    <>
      {inPane ? (
        <InPaneHead slot={paneHeadSlot.title}>
          {top}
          {introLine}
        </InPaneHead>
      ) : null}
      {inPane && empty ? null : (
        <section {...partOf(part.card, { variants: [look] })} aria-labelledby={headingId}>
          {inPane ? null : top}
          <div {...partOf(part.stack)}>
            {inPane ? null : introLine}
            {children}
            {backLink}
          </div>
        </section>
      )}
    </>
  );
}
