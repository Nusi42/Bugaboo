import type { Schema } from "@bugaboo/api-client";
import { t, type MessageKey } from "@bugaboo/messages";
import { Alert } from "@bugaboo/ui";

import styles from "./Admin.module.css";
import { isUnanchored } from "./status.ts";

type SecurityLog = Schema<"SecurityLogBody">;
type Reason = NonNullable<SecurityLog["reason"]>;

/** The sentence for why a chain of the security log breaks, by the reason the server names. */
const reasonMessages: Readonly<Record<Reason, MessageKey>> = {
  gap: "security-log.reason.gap",
  broken_link: "security-log.reason.broken-link",
  changed_content: "security-log.reason.changed-content",
  checkpoint: "security-log.reason.checkpoint",
  truncated: "security-log.reason.truncated",
  anchor: "security-log.reason.anchor",
};

/** The result of checking the security log, in words. */
export function LogStatus({ log }: { readonly log: SecurityLog }) {
  if (log.result === "broken") {
    const reason =
      log.reason === null || log.reason === undefined ? undefined : reasonMessages[log.reason];
    return (
      <Alert danger>
        {t("security-log.broken", {
          seq: log.broken_at ?? 0,
          reason: reason === undefined ? "" : t(reason),
        })}
      </Alert>
    );
  }
  if (log.result === "unreadable") {
    return <Alert danger>{t("security-log.unreadable")}</Alert>;
  }
  return (
    <div className={styles.detail}>
      <span>{t("security-log.valid", { entries: log.entries ?? 0 })}</span>
      {log.head_seq === null || log.head_seq === undefined ? null : (
        <span>{t("security-log.head", { seq: log.head_seq, hash: log.head_hash ?? "" })}</span>
      )}
      {isUnanchored(log) ? <Alert warning>{t("security-log.unanchored")}</Alert> : null}
    </div>
  );
}
