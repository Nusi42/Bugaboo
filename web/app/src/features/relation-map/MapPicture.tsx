import { t } from "@bugaboo/messages";
import { part, partOf, pixelsOf, usePlace } from "@bugaboo/ui";
import { useLayoutEffect, useRef } from "react";

import { useBackgroundDrag } from "./background-drag.ts";
import { MapCardButton, type ChainMark, type Pointing } from "./MapCardButton.tsx";
import {
  cardPlace,
  drawnLine,
  headPlace,
  pictureSize,
  type DrawnLine,
  type MapSizes,
  type Point,
} from "./geometry.ts";
import { hasDirection, lookOfLine, type LineLook, type Looks } from "./line-look.ts";
import { onChainOf, type MapPicture as Picture, type OnChain } from "./layout.ts";

export interface MapPictureProps extends Pointing {
  readonly picture: Picture;
  readonly sizes: MapSizes;
  readonly looks: Looks;
  /** 1 is the size of the text. */
  readonly zoom: number;
  /** The issue pointed at, whose chain is shown for the moment. */
  readonly pointed: string | undefined;
  /** The issue chosen, whose chain stays shown. */
  readonly chosen: string | undefined;
  /** What the picture is for screen readers: one figure, with the list as its reading order. */
  readonly label: string;
  /** Whether the picture is to be fitted to its box now (`onFit` says the zoom, which clears it). */
  readonly fitDue: boolean;
  /** The zoom at which the whole picture fits into its box. */
  readonly onFit: (zoom: number) => void;
}

/** A line as it is drawn, with the look of its kind. */
interface LookedLine {
  readonly each: DrawnLine;
  readonly look: LineLook;
}

/** Something that stands on a place of the picture: a word of a line (`on` its shown chain), or the name of a column. */
function Placed({
  at,
  label,
  on,
  text,
}: {
  readonly at: Point;
  readonly label: boolean;
  readonly on: boolean;
  readonly text: string;
}) {
  const node = useRef<HTMLSpanElement>(null);
  usePlace(node, { "map-x": at.x, "map-y": at.y });
  return (
    <span
      ref={node}
      aria-hidden
      {...partOf(label ? part.mapLabel : part.mapHead, { states: [label && on && "on-chain"] })}
    >
      {text}
    </span>
  );
}

/** The name of a column after the start: the steps away. */
function headOf(column: number): string {
  return t("relation-map.column", { count: column });
}

/** The names of the columns after the start, which its card names itself (its tab *Start*). */
function ColumnHeads({ picture, sizes }: Pick<MapPictureProps, "picture" | "sizes">) {
  return picture.columns.slice(1).map((cards, before) => {
    const column = before + 1;
    return (
      <Placed
        key={cards[0]?.issue.id ?? headOf(column)}
        at={headPlace(column, sizes)}
        label={false}
        on={false}
        text={headOf(column)}
      />
    );
  });
}

/** How a line or a card stands while a chain is shown. */
function markOf(on: boolean, shown: boolean): ChainMark {
  if (!shown) {
    return undefined;
  }
  return on ? "on-chain" : "faded";
}

/** A line and its arrow: its word is a part of its own, above every line. */
function Line({ each, look, mark }: LookedLine & { readonly mark: ChainMark }) {
  return (
    <g {...partOf(part.mapLine, { variants: [look], states: [mark] })}>
      <path d={each.path} />
      {each.head === undefined ? null : <path {...partOf(part.mapLineHead)} d={each.head} />}
    </g>
  );
}

/** What the figure holds: the lines, the names of the columns, the words of the lines and the cards. */
function Stage(
  props: MapPictureProps & {
    readonly drawn: readonly LookedLine[];
    readonly chain: OnChain | undefined;
  },
) {
  const { picture, sizes, chosen, drawn, chain } = props;
  const shown = chain !== undefined;
  const lineOn = (key: string): boolean => chain?.lines.includes(key) === true;
  return (
    <figure {...partOf(part.mapStage)} aria-label={props.label}>
      <svg {...partOf(part.mapLines)} aria-hidden>
        {drawn.map(({ each, look }) => (
          <Line key={each.key} each={each} look={look} mark={markOf(lineOn(each.key), shown)} />
        ))}
      </svg>
      {picture.start === undefined ? null : <ColumnHeads picture={picture} sizes={sizes} />}
      {drawn.map(({ each }) => (
        <Placed
          key={each.key}
          at={each.label}
          label
          on={lineOn(each.key)}
          text={each.line.forward}
        />
      ))}
      {Object.values(picture.cards).map((card) => (
        <MapCardButton
          key={card.issue.id}
          {...props}
          issue={card.issue}
          place={cardPlace(card, sizes)}
          start={card.issue.id === picture.start}
          chosen={card.issue.id === chosen}
          mark={markOf(chain?.issues.includes(card.issue.id) === true, shown)}
        />
      ))}
    </figure>
  );
}

/**
 * The picture of the relation map (UX §4.15): the cards where the layout puts them, a line for
 * each relation in the look of its kind with its word on its middle, and the names of the columns
 * on the map of an issue. Pointing at a card (or giving it the focus) shows its chain back to the
 * start and fades the rest; only colors change, so nothing moves under the cursor (UX §6.4).
 */
export function MapPicture(props: MapPictureProps) {
  const { picture, sizes, looks, zoom, pointed, chosen, fitDue, onFit } = props;
  const box = useRef<HTMLDivElement>(null);
  const canvas = useRef<HTMLDivElement>(null);
  const size = pictureSize(picture, sizes);
  // *Fit*: the zoom at which the picture and the room around it fit the box, measured once for each
  // press; a new answer keeps the zoom the person has.
  // Before the picture is painted, so the first frame already has the zoom it keeps.
  useLayoutEffect(() => {
    const element = box.current;
    if (!fitDue || element === null || size.width <= 0 || size.height <= 0) {
      return;
    }
    const room = pixelsOf("space-5") * 2;
    onFit(
      Math.min(
        (element.clientWidth - room) / size.width,
        (element.clientHeight - room) / size.height,
      ),
    );
  }, [fitDue, onFit, size.width, size.height]);
  usePlace(canvas, { "map-width": size.width, "map-height": size.height, "map-zoom": zoom });
  const active = pointed ?? chosen;
  const drag = useBackgroundDrag();
  const drawn: readonly LookedLine[] = picture.lines.flatMap((line) => {
    const look = lookOfLine(line, looks);
    const each = drawnLine(picture, line, sizes, hasDirection(look));
    return each === undefined ? [] : [{ each, look }];
  });
  return (
    <div
      ref={box}
      {...partOf(part.mapScroll, { states: [drag.dragging && "dragging"] })}
      {...drag.handlers}
    >
      <div ref={canvas} {...partOf(part.mapCanvas)}>
        <Stage
          {...props}
          drawn={drawn}
          chain={active === undefined ? undefined : onChainOf(picture, active)}
        />
      </div>
    </div>
  );
}
