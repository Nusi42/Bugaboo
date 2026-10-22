import { Alert, alertTone, partOf, part } from "@bugaboo/ui";
import type { ReactNode } from "react";

import { messageOf } from "../../shared/errors.ts";
import { OfferedHere } from "./OfferedHere.tsx";

/** What the project's screen needs to know of one thing it offers (an issue type, a group of labels). */
export interface ProjectOffer {
  readonly id: string;
  readonly name: string;
  readonly beside?: ReactNode;
  /** Whether the project offers it; what the server does not say counts as offered. */
  readonly offered: boolean | null | undefined;
  /** The line the project has of its own; none while it follows the project above. */
  readonly own: boolean | null | undefined;
  readonly mayChange: boolean | null | undefined;
}

export interface ProjectOffersProps {
  /** The name of the list for screen readers. */
  readonly label: string;
  readonly offers: readonly ProjectOffer[];
  readonly isPending: boolean;
  /** Why the last change did not work, if it did not. */
  readonly error: unknown;
  /** The person chose for one thing: offered, not offered, or the project above again (`null`). */
  readonly onChange: (id: string, offered: boolean | null) => void;
}

/**
 * The list of what a project offers, each with a switch: the same list for the issue types and the
 * groups of labels (ISS-02, TAG-02), so both read the server's answer one way.
 */
export function ProjectOffers({ label, offers, isPending, error, onChange }: ProjectOffersProps) {
  return (
    <div {...partOf(part.stack)}>
      <ul {...partOf(part.list)} aria-label={label}>
        {offers.map((offer) => (
          <OfferedHere
            key={offer.id}
            name={offer.name}
            beside={offer.beside}
            offered={offer.offered ?? true}
            own={offer.own ?? undefined}
            mayChange={offer.mayChange === true}
            isPending={isPending}
            onChange={(offered) => {
              onChange(offer.id, offered);
            }}
          />
        ))}
      </ul>
      {error === null || error === undefined ? null : (
        <Alert tone={alertTone.danger}>{messageOf(error)}</Alert>
      )}
    </div>
  );
}
