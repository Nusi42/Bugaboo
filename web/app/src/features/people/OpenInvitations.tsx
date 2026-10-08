import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, buttonLook, ConfirmDialog } from "@bugaboo/ui";
import { useState } from "react";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { formatDateTime } from "../../shared/format.ts";
import styles from "./People.module.css";
import { refreshInvitations, useOpenInvitations } from "./invitations.ts";
import { linesFrom } from "../../shared/paged.ts";

type Invitation = Schema<"InvitationItem">;

/** One open invitation, with a way to take it back. */
interface InvitationRowProps {
  readonly invitation: Invitation;
  readonly onTakenBack: () => void;
}

function InvitationRow({ invitation, onTakenBack }: InvitationRowProps) {
  const { id, email, created_at: made, expires_at: expires } = invitation;
  const revoke = api.query.useMutation("delete", "/api/v1/auth/invitations/{invitation_id}", {
    onSuccess: () => {
      refreshInvitations();
      onTakenBack();
    },
  });
  if (id === null || id === undefined || email === null || email === undefined) {
    return null;
  }
  return (
    <li className={styles.item}>
      <span className={styles.itemText}>
        <span className={styles.itemTitle}>{email}</span>
        {made === null || made === undefined ? null : (
          <span className={styles.muted}>
            {t("invitations.made", { when: formatDateTime(made) })}
          </span>
        )}
        {expires === null || expires === undefined ? null : (
          <span className={styles.muted}>
            {t("invitations.expires", { when: formatDateTime(expires) })}
          </span>
        )}
        {revoke.isError ? <Alert tone={alertTone.danger}>{messageOf(revoke.error)}</Alert> : null}
      </span>
      <ConfirmDialog
        triggerLabel={t("invitations.take-back")}
        triggerName={t("invitations.take-back-name", { email })}
        title={t("invitations.take-back-title", { email })}
        text={t("invitations.take-back-text")}
        confirmLabel={t("invitations.take-back")}
        cancelLabel={t("common.cancel")}
        confirmLook={buttonLook.danger}
        onConfirm={() => {
          revoke.mutate({ params: { path: { invitation_id: id } } });
        }}
      />
    </li>
  );
}

/** The invitations that nobody has used yet and that have not run out. */
export function OpenInvitations() {
  const invitations = useOpenInvitations();
  const [takenBack, setTakenBack] = useState(false);
  const items = linesFrom(invitations.data?.pages, (item) => item);
  if (invitations.isError) {
    return <Alert tone={alertTone.danger}>{messageOf(invitations.error)}</Alert>;
  }
  if (invitations.isSuccess && items.length === 0) {
    return <p className={styles.muted}>{t("invitations.none")}</p>;
  }
  return (
    <>
      {takenBack ? <Alert tone={alertTone.success}>{t("invitations.taken-back")}</Alert> : null}
      <ul className={styles.list} aria-label={t("invitations.open")}>
        {items.map((invitation) => (
          <InvitationRow
            key={invitation.id}
            invitation={invitation}
            onTakenBack={() => {
              setTakenBack(true);
            }}
          />
        ))}
      </ul>
      {invitations.hasNextPage ? (
        <div className={styles.row}>
          <Button
            isPending={invitations.isFetchingNextPage}
            onPress={() => {
              void invitations.fetchNextPage();
            }}
          >
            {t("invitations.more")}
          </Button>
        </div>
      ) : null}
    </>
  );
}
