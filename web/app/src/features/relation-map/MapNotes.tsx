import { t } from "@bugaboo/messages";
import { Alert, alertTone, BottomSheet, part, partOf } from "@bugaboo/ui";

import { loopSteps, type MapAnswer, type MapPicture as Picture } from "./layout.ts";
import { ChainPanel } from "./MapSide.tsx";
import { loopSentence } from "./words.ts";

/** The notes over the map: that it was cut, that it holds no relation, and each loop with its sentence. */
export function MapNotes({
  answer,
  picture,
  cut,
  nothing,
}: {
  readonly answer: MapAnswer;
  /** What the map says when it was cut at the most issues it holds. */
  readonly cut: string;
  /** What the map says when it holds no relation of the kinds shown. */
  readonly nothing: string;
} & { readonly picture: Picture }) {
  return (
    <>
      {answer.truncated ? <Alert tone={alertTone.warning}>{cut}</Alert> : null}
      {picture.lines.length === 0 ? <p {...partOf(part.muted)}>{nothing}</p> : null}
      {answer.loops.map((loop) => {
        const sentence = loopSentence(loopSteps(picture, loop) ?? []);
        return sentence === undefined ? null : (
          <Alert key={loop.join(",")} tone={alertTone.warning}>
            {sentence}
          </Alert>
        );
      })}
    </>
  );
}

/** The chain of the chosen issue on a phone: a sheet from the bottom. */
export function PhoneChain({
  picture,
  chosen,
  onStartFrom,
  onClose,
}: {
  readonly picture: Picture;
  readonly chosen: string | undefined;
  readonly onStartFrom: (key: string) => void;
  readonly onClose: () => void;
}) {
  return (
    <BottomSheet
      isOpen={chosen !== undefined}
      onOpenChange={(open) => {
        if (!open) {
          onClose();
        }
      }}
      label={
        chosen === undefined
          ? t("relation-map.chain-hint")
          : t("relation-map.chain", { key: picture.cards[chosen]?.issue.readable_id ?? chosen })
      }
      closeLabel={t("common.close")}
    >
      <ChainPanel picture={picture} chosen={chosen} onStartFrom={onStartFrom} />
    </BottomSheet>
  );
}
