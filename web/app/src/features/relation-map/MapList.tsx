import { t } from "@bugaboo/messages";
import { ListGrid, part, partOf } from "@bugaboo/ui";
import { useId } from "react";

import { IssueRefRow } from "../relations/index.ts";
import { chainOf, type MapCard, type MapPicture } from "./layout.ts";
import { leadOf } from "./words.ts";

export interface MapListProps {
  readonly picture: MapPicture;
}

/** The heading of a group: the start, the first issues of the groups of a search, or the steps away. */
function groupName(picture: MapPicture, column: number): string {
  if (column > 0) {
    return t("relation-map.column", { count: column });
  }
  return picture.start === undefined ? t("relation-map.first-column") : t("relation-map.start");
}

/** One issue of the list: the sentence of its chain up to it, then the issue as a row, which opens it. */
function ChainRow({ picture, card }: { readonly picture: MapPicture; readonly card: MapCard }) {
  const lead = leadOf(chainOf(picture, card.issue.id));
  return <IssueRefRow issue={card.issue} meaning={lead === "" ? undefined : lead} />;
}

/** One group of the list: the issues of one column of the picture, in its order. */
function Group({
  picture,
  cards,
  column,
}: MapListProps & { readonly cards: readonly MapCard[]; readonly column: number }) {
  const heading = useId();
  const name = groupName(picture, column);
  return (
    <section aria-labelledby={heading}>
      <h3 id={heading} {...partOf(part.mapListHeading)}>
        {name}
      </h3>
      <ListGrid label={name}>
        {cards.map((card) => (
          <ChainRow key={card.issue.id} picture={picture} card={card} />
        ))}
      </ListGrid>
    </section>
  );
}

/**
 * The list form of the map (UX §4.15): the same chains as the picture, as sentences grouped by
 * steps away, each issue with its status icon and the edge of its priority. Phones show it first,
 * and it is what screen readers read: the picture points here.
 */
export function MapList({ picture }: MapListProps) {
  return (
    <section {...partOf(part.mapList)} aria-label={t("relation-map.list-label")}>
      {picture.columns.map((cards, column) => (
        <Group
          key={cards[0]?.issue.id ?? groupName(picture, column)}
          picture={picture}
          cards={cards}
          column={column}
        />
      ))}
    </section>
  );
}
