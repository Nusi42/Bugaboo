import type { Wording } from "@bugaboo/wasm";
import type { ReactNode } from "react";

import { lengthReaderOf, type LengthReader } from "./length.ts";
import { WithWording, WordingNotice } from "./with-wording.tsx";

export type { LengthReader } from "./length.ts";

/**
 * A part of a screen that reads a typed length of time with the one reader of `bb-core`: the module
 * is loaded when it opens, a skeleton stands in until then, and a notice with *Try again* says so
 * when it could not be loaded.
 */
export function WithLengthReader({
  children,
  dayMinutes,
}: {
  readonly children: (readLength: LengthReader, wording: Wording) => ReactNode;
  /** The minutes of a work day of the person the length is for: days count in them. */
  readonly dayMinutes?: number | undefined;
}) {
  return (
    <>
      <WordingNotice />
      <WithWording>
        {(wording) => children(lengthReaderOf(wording, dayMinutes), wording)}
      </WithWording>
    </>
  );
}
