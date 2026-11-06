import { secondsPerMinute } from "@bugaboo/api-client";
import { partOf, part } from "@bugaboo/ui";

import { WithLengthReader } from "../../shared/with-length-reader.tsx";
import type { BoxField } from "./fields.ts";
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
  /** A hint under the box, for a box of a form: what can be typed. */
  readonly help?: string;
  /** What the server said was wrong with the length, in words: shown at the box. */
  readonly problem?: string | undefined;
  /** Told when the text typed is no length, for a form that must not send it until it is mended. */
  readonly onMistake?: () => void;
}

/**
 * A box in a row of the Details for a length of the issue itself, in whole minutes (*Estimated
 * time*, *Time left*, TIME-03): the same box, reader and words as the length of an own field, so a
 * length is typed and written one way everywhere.
 */
export function LengthBox({
  name,
  minutes,
  onCommit,
  inRow = true,
  help = "",
  problem,
  onMistake,
}: LengthBoxProps) {
  const field: BoxField = { name, kind: "duration", help, rules: { required: false } };
  return (
    <span {...partOf(part.lengthBox)}>
      <WithLengthReader>
        {(readLength) => (
          <TextEditor
            field={field}
            value={minutes === undefined ? undefined : minutes * secondsPerMinute}
            inRow={inRow}
            live={!inRow}
            problem={problem}
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
