import { secondsPerMinute } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";
import type { Wording } from "@bugaboo/wasm";

import { WithLengthReader } from "../../shared/with-length-reader.tsx";
import type { BoxField } from "./fields.ts";
import { LengthHint } from "./LengthHint.tsx";
import { TextEditor } from "./TextEditor.tsx";

export interface LengthBoxProps {
  /** The name of the length, which the row shows and the box reads out. */
  readonly name: string;
  /** The minutes now; none while it has no value. */
  readonly minutes: number | undefined;
  /** Sends the minutes typed; `null` takes the value away. */
  readonly onCommit: (minutes: number | null) => void;
  /**
   * The box stands in a row of the Details, which shows the name (the default). Without it the box
   * has a visible name and sends every change at once, as a box of a form does.
   */
  readonly inRow?: boolean;
  /** What the server said was wrong with the length, in words: shown at the box. */
  readonly problem?: string | undefined;
  /** Told when the text typed is no length, for a form that must not send it until it is mended. */
  readonly onMistake?: () => void;
}

/**
 * The box of a length: in a row of the Details it writes the short form (`1h 30m`, UX §4.9); a box of
 * a form (*Add time*, *Stop timer*) writes the grid form (`1:30`), the form of the timesheet.
 */
function fieldOf(name: string, inRow: boolean, wording: Wording): BoxField {
  const rules = { required: false };
  return inRow
    ? { name, kind: "duration", rules }
    : {
        name,
        kind: "duration",
        rules,
        lengthText: (seconds) => wording.lengthOnClock(Math.floor(seconds / secondsPerMinute)),
      };
}

/**
 * A box in a row of the Details for a length of the issue itself, in whole minutes (*Estimated
 * time*, *Time left*, TIME-03): the same box, reader and words as the length of an own field, so a
 * length is typed and written one way everywhere. A box of a form (not in a row) says under itself
 * what can be typed; a box in a row of the Details keeps no room for a hint.
 */
export function LengthBox({
  name,
  minutes,
  onCommit,
  inRow = true,
  problem,
  onMistake,
}: LengthBoxProps) {
  return (
    <span {...partOf(part.lengthBox)}>
      <WithLengthReader>
        {(readLength, wording) => (
          <TextEditor
            field={fieldOf(name, inRow, wording)}
            value={minutes === undefined ? undefined : minutes * secondsPerMinute}
            inRow={inRow}
            live={!inRow}
            problem={problem}
            hint={inRow ? undefined : <LengthHint />}
            {...(inRow ? { placeholder: t("fields.length-add") } : {})}
            readLength={readLength}
            {...(onMistake === undefined ? {} : { onMistake })}
            onCommit={(sent) => {
              onCommit(typeof sent === "number" ? Math.floor(sent / secondsPerMinute) : null);
            }}
          />
        )}
      </WithLengthReader>
    </span>
  );
}
