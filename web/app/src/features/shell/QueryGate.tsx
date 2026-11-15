import { t } from "@bugaboo/messages";
import { Alert, AnswerNote, alertTone, partOf, part } from "@bugaboo/ui";
import { type QueryKey, useQueryClient } from "@tanstack/react-query";
import type { ReactNode } from "react";

import { codeOf, isRetryable, messageOf, supportCodeOf } from "../../shared/errors.ts";
import { useKeptError } from "../../shared/use-kept-error.ts";

export interface QueryGateProps<Data> {
  /** Why the query failed, if it did (`query.error`). */
  readonly error: unknown;
  /** What the page needs once it is there; nothing while it loads. */
  readonly data: Data | undefined;
  /**
   * What shows when the server says the thing is not there (or not for this person, which looks
   * the same). Without it, that answer is told like any other failure.
   */
  readonly notFound?: ReactNode;
  /** What shows while the data loads; the word *Loading* by default. */
  readonly loading?: ReactNode;
  /**
   * The key of the read, where one gate shows reads of several addresses (an issue, a project): the
   * failure is kept while the same read is done again, never for another one. Without it the gate
   * stands for one read.
   */
  readonly read?: QueryKey;
  /** The page, once the data is there. */
  readonly children: (data: Readonly<Data>) => ReactNode;
}

/** The key of the read of a gate that shows one read only. */
const ONE_READ: QueryKey = ["query-gate"];

/**
 * The part of a page that comes before the page: a failure is told in plain words (a thing that is
 * not there, as `notFound`), and while the data loads nothing is claimed.
 */
export function QueryGate<Data>({
  error,
  data,
  notFound,
  loading,
  read = ONE_READ,
  children,
}: QueryGateProps<Data>) {
  const client = useQueryClient();
  // A failure stays on screen while the read is done again in the background (UX §6.4).
  const failed = useKeptError({ queryKey: read, data, error });
  if (failed !== null && failed !== undefined) {
    if (notFound !== undefined && codeOf(failed) === "resource.not-found") {
      return notFound;
    }
    const code = supportCodeOf(failed);
    return isRetryable(failed) ? (
      <AnswerNote
        tone={alertTone.danger}
        text={
          code === undefined
            ? messageOf(failed)
            : t("error.with-support-code", { message: messageOf(failed), code })
        }
        main={{
          label: t("common.try-again"),
          onPress: () => {
            void client.refetchQueries({ stale: true });
          },
        }}
      />
    ) : (
      <Alert tone={alertTone.danger}>{messageOf(failed)}</Alert>
    );
  }
  if (data === undefined) {
    return loading === undefined ? <p {...partOf(part.muted)}>{t("common.loading")}</p> : loading;
  }
  return children(data);
}
