import { t } from "@bugaboo/messages";
import { ChoiceGroup, TextField } from "@bugaboo/ui";

import { formatNumber } from "../../shared/format.ts";
import { LengthBox } from "../fields/index.ts";
import { TimeFactorSum } from "./TimeFactorSum.tsx";
import type { TimeFactorDraft } from "./use-time-factor-draft.ts";

/** An example of a factor, written as the person writes numbers, for the sentence of a mistake. */
const EXAMPLE_FACTOR = 1.25;

export interface TimeFactorFieldsProps {
  readonly draft: TimeFactorDraft;
  /** *Estimated time* in minutes when the reader may read it: only then a length can be given. */
  readonly estimate: number | undefined;
  /** The person on the bar by name when they are somebody else; none for one's own factor. */
  readonly otherName: string | undefined;
}

/**
 * The boxes of the time factor: the choice between a length (*Takes me …*) and a factor, where the
 * estimate can be read, then the box of the one chosen and the sum as it is typed.
 */
export function TimeFactorFields({ draft, estimate, otherName }: TimeFactorFieldsProps) {
  return (
    <>
      {estimate === undefined ? null : (
        <ChoiceGroup
          legend={t("issues.time-factor-way")}
          choices={[
            {
              value: "length",
              label:
                otherName === undefined
                  ? t("issues.time-factor-takes-me")
                  : t("issues.time-factor-takes", { name: otherName }),
            },
            { value: "factor", label: t("issues.time-factor-as-factor") },
          ]}
          value={draft.way}
          onChange={draft.setWay}
        />
      )}
      {draft.way === "length" ? (
        <LengthBox
          name={t("issues.time-factor-length")}
          minutes={draft.minutes ?? undefined}
          inRow={false}
          onCommit={draft.setMinutes}
        />
      ) : (
        <TextField
          label={t("issues.time-factor-factor")}
          value={draft.typed}
          onChange={draft.setTyped}
          errorMessage={
            draft.typed.trim() === "" || draft.typedPercent !== null
              ? undefined
              : t("issues.time-factor-not-a-factor", { example: formatNumber(EXAMPLE_FACTOR) })
          }
        />
      )}
      <TimeFactorSum
        estimate={estimate}
        percent={draft.way === "factor" ? draft.typedPercent : null}
        minutes={draft.way === "length" ? draft.minutes : null}
      />
    </>
  );
}
