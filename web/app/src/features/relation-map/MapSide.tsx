import { t } from "@bugaboo/messages";
import { Button, Legend, LegendItem, Link, live, part, partOf } from "@bugaboo/ui";
import { useId } from "react";

import { issueAddress } from "../../shared/paths.ts";
import { IssueLegend } from "../issues/index.ts";
import type { RelationKindLine } from "../relation-kinds/index.ts";
import { chainOf, type MapPicture } from "./layout.ts";
import { swatchOf, type Looks } from "./line-look.ts";
import { sentenceOf, stepSentence } from "./words.ts";

export interface ChainPanelProps {
  readonly picture: MapPicture;
  /** The issue chosen; none until one is clicked. */
  readonly chosen: string | undefined;
  /** Makes an issue the start of a new map, by its readable ID. */
  readonly onStartFrom: (key: string) => void;
}

/**
 * The chain of the chosen issue (UX §4.15): how it connects back to the start, as one sentence and
 * as numbered relations ("WEB-142 blocks WEB-147, which is blocked by WEB-148"), with *Start from
 * here* and *Open issue*. Before an issue is chosen it says how to choose one.
 */
export function ChainPanel({ picture, chosen, onStartFrom }: ChainPanelProps) {
  const heading = useId();
  const card = chosen === undefined ? undefined : picture.cards[chosen];
  if (card === undefined) {
    return (
      <section {...partOf(part.mapChain)} aria-live={live.polite}>
        <p {...partOf(part.muted)}>{t("relation-map.chain-hint")}</p>
      </section>
    );
  }
  const key = card.issue.readable_id;
  const steps = chainOf(picture, card.issue.id);
  const isStart = card.issue.id === picture.start;
  const lone = isStart
    ? t("relation-map.chain-start", { key })
    : t("relation-map.chain-first", { key });
  return (
    <section {...partOf(part.mapChain)} aria-labelledby={heading} aria-live={live.polite}>
      <h2 id={heading} {...partOf(part.mapChainHeading)}>
        {t("relation-map.chain", { key })}
      </h2>
      <p>{sentenceOf(steps) ?? lone}</p>
      {steps.length < 2 ? null : (
        <ol {...partOf(part.mapChainSteps)}>
          {steps.map((step) => (
            <li key={step.to.id}>{stepSentence(step)}</li>
          ))}
        </ol>
      )}
      <div {...partOf(part.mapChainActions)}>
        {isStart ? null : (
          <Button
            onPress={() => {
              onStartFrom(key);
            }}
          >
            {t("relation-map.start-from-here")}
          </Button>
        )}
        <Link href={issueAddress(key)} button>
          {t("relation-map.open-issue")}
        </Link>
      </div>
    </section>
  );
}

/** What the lines mean: each kind of relation in its look, where its arrow points, and *part of*. */
export function LineLegend({
  kinds,
  looks,
}: {
  readonly kinds: readonly RelationKindLine[];
  readonly looks: Looks;
}) {
  return (
    <Legend label={t("relation-map.legend")}>
      {kinds.map((kind) => {
        const look = looks.ofKind(kind.id);
        return (
          <LegendItem key={kind.id} look={swatchOf(look)}>
            {look === "alike"
              ? t("relation-map.legend-alike", { word: kind.forward })
              : t("relation-map.legend-arrow", { word: kind.forward, other: kind.backward })}
          </LegendItem>
        );
      })}
      <LegendItem look={swatchOf(looks.partOf)}>{t("relation-map.legend-part-of")}</LegendItem>
    </Legend>
  );
}

/** The side of the picture on a computer: the chain of the chosen issue, what the lines mean, and the card colors. */
export function MapSide(
  props: ChainPanelProps & {
    readonly kinds: readonly RelationKindLine[];
    readonly looks: Looks;
  },
) {
  return (
    <aside {...partOf(part.mapSide)}>
      <ChainPanel {...props} />
      <LineLegend kinds={props.kinds} looks={props.looks} />
      <IssueLegend />
    </aside>
  );
}
