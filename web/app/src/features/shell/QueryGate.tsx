import { t } from "@bugaboo/messages";
import { Alert, alertTone } from "@bugaboo/ui";
import type { ReactNode } from "react";

import { codeOf, messageOf } from "../../shared/errors.ts";
import styles from "./Shell.module.css";

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
  if (error !== null && error !== undefined) {
    return notFound !== undefined && codeOf(error) === "resource.not_found" ? (
      notFound
    ) : (
      <Alert tone={alertTone.danger}>{messageOf(error)}</Alert>
    );
  }
  if (data === undefined) {
    return loading === undefined ? <p className={styles.muted}>{t("common.loading")}</p> : loading;
  }
  return children(data);
}
