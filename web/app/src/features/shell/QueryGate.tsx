import { t } from "@bugaboo/messages";
import { Alert, AnswerNote, alertTone, partOf, part } from "@bugaboo/ui";
import { useQueryClient } from "@tanstack/react-query";
import type { ReactNode } from "react";

import { codeOf, isRetryable, messageOf, supportCodeOf } from "../../shared/errors.ts";

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
  /** The page, once the data is there. */
  readonly children: (data: Readonly<Data>) => ReactNode;
}

/**
 * The part of a page that comes before the page: a failure is told in plain words (a thing that is
 * not there, as `notFound`), and while the data loads nothing is claimed.
 */
export function QueryGate<Data>({
  error,
  data,
  notFound,
  loading,
  children,
}: QueryGateProps<Data>) {
  const client = useQueryClient();
  if (error !== null && error !== undefined) {
    if (notFound !== undefined && codeOf(error) === "resource.not-found") {
      return notFound;
    }
    const code = supportCodeOf(error);
    return isRetryable(error) ? (
      <AnswerNote
        tone={alertTone.danger}
        text={
          code === undefined
            ? messageOf(error)
            : t("error.with-support-code", { message: messageOf(error), code })
        }
        main={{
          label: t("common.try-again"),
          onPress: () => {
            void client.refetchQueries({ stale: true });
          },
        }}
      />
    ) : (
      <Alert tone={alertTone.danger}>{messageOf(error)}</Alert>
    );
  }
  if (data === undefined) {
    return loading === undefined ? <p {...partOf(part.muted)}>{t("common.loading")}</p> : loading;
  }
  return children(data);
}
