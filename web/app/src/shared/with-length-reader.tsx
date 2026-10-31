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
}: {
  readonly children: (readLength: LengthReader) => ReactNode;
}) {
  return (
    <>
      <WordingNotice />
      <WithWording>{(wording) => children(lengthReaderOf(wording))}</WithWording>
    </>
  );
}
