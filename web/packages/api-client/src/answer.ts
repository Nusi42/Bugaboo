import { ApiError } from "./problem.ts";

/** What the client gives back: the answer, or nothing (for an answer without a body). */
export interface Answer<Data> {
  readonly data?: Data | undefined;
}

/**
 * The body of an answer. A request that fails rejects before this (the client raises an
 * {@link ApiError}); an answer without a body is none the caller can use, so it is treated as no
 * answer at all.
 */
export function dataOf<Data>(answer: Answer<Data>): Data {
  if (answer.data === undefined) {
    throw new ApiError(0, undefined);
  }
  return answer.data;
}
