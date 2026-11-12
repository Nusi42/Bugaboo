import { t } from "@bugaboo/messages";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { formatNumber } from "../../shared/format.ts";
import { hundredthsIn } from "../../shared/number.ts";
import { AS_ESTIMATED } from "../../shared/time-factor-text.ts";
import { setTimeFactor, type NewTimeFactor, type TimeFactor } from "./time-factor.ts";

/** How the factor is given: a length (*Takes me …*) or a factor. */
export type FactorWay = "length" | "factor";

/** What the window of the time factor holds while it is filled in, and what saving it does. */
export interface TimeFactorDraft {
  readonly way: FactorWay;
  readonly setWay: (way: FactorWay) => void;
  /** The length typed, in minutes; none while nothing is typed. */
  readonly minutes: number | null;
  readonly setMinutes: (minutes: number | null) => void;
  /** The factor as typed (`1.25`), and what it is in percent, or `null` when it is no factor. */
  readonly typed: string;
  readonly setTyped: (text: string) => void;
  readonly typedPercent: number | null;
  /** The plain message of a refusal, or of a window sent with nothing in it. */
  readonly problem: string | null;
  /** Sends the factor; resolves to whether it was saved. */
  readonly save: () => Promise<boolean>;
}

/** The new factor the draft gives, or none while its box holds nothing that can be sent. */
function givenOf(
  way: FactorWay,
  minutes: number | null,
  typedPercent: number | null,
): NewTimeFactor | null {
  if (way === "length") {
    return minutes === null ? null : { takes_minutes: minutes };
  }
  return typedPercent === null ? null : { percent: typedPercent };
}

/**
 * The draft of the window: a length is offered first where the estimate can be read (`withLength`),
 * the factor box starts with the factor now. The server works the length out against the estimate.
 */
export function useTimeFactorDraft(
  readableId: string,
  factor: TimeFactor,
  percent: number,
  withLength: boolean,
): TimeFactorDraft {
  const [way, setWay] = useState<FactorWay>(withLength ? "length" : "factor");
  const [minutes, setMinutes] = useState<number | null>(null);
  const [typed, setTyped] = useState(formatNumber(percent / AS_ESTIMATED));
  const [problem, setProblem] = useState<string | null>(null);
  const typedPercent = hundredthsIn(typed);
  return {
    way,
    setWay,
    minutes,
    setMinutes,
    typed,
    setTyped,
    typedPercent,
    problem,
    save: async () => {
      const given = givenOf(way, minutes, typedPercent);
      if (given === null) {
        setProblem(t("issues.time-factor-nothing"));
        return false;
      }
      try {
        await setTimeFactor(readableId, factor.version, given);
        return true;
      } catch (error) {
        setProblem(messageOf(error));
        return false;
      }
    },
  };
}
