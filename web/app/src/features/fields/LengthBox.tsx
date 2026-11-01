import { secondsPerMinute } from "@bugaboo/api-client";

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
}

/**
 * A box in a row of the Details for a length of the issue itself, in whole minutes (*Estimated
 * time*, *Time left*, TIME-03): the same box, reader and words as the length of an own field, so a
 * length is typed and written one way everywhere.
 */
export function LengthBox({ name, minutes, onCommit }: LengthBoxProps) {
  const field: BoxField = { name, kind: "duration", help: "", rules: { required: false } };
  return (
    <WithLengthReader>
      {(readLength) => (
        <TextEditor
          field={field}
          value={minutes === undefined ? undefined : minutes * secondsPerMinute}
          inRow
          readLength={readLength}
          onCommit={(sent) => {
            onCommit(typeof sent === "number" ? Math.floor(sent / secondsPerMinute) : null);
          }}
        />
      )}
    </WithLengthReader>
  );
}
