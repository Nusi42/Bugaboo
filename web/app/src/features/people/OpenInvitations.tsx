import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, alertTone, buttonLook, ConfirmDialog, partOf, part, ShowMoreOf } from "@bugaboo/ui";
import { useState } from "react";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { formatDateTime } from "../../shared/format.ts";
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
  const revoke = api.query.useMutation("delete", "/api/v1/auth/invitations/{invitation}", {
    onSuccess: () => {
      refreshInvitations();
      onTakenBack();
    },
  });
  if (id === null || id === undefined || email === null || email === undefined) {
    return null;
  }
  return (
    <li {...partOf(part.item)}>
      <span {...partOf(part.itemText)}>
        <span {...partOf(part.itemTitle)}>{email}</span>
        {made === null || made === undefined ? null : (
          <span {...partOf(part.muted)}>
            {t("invitations.created-at", { when: formatDateTime(made) })}
          </span>
        )}
        {expires === null || expires === undefined ? null : (
          <span {...partOf(part.muted)}>
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
          revoke.mutate({ params: { path: { invitation: id } } });
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
    return <p {...partOf(part.muted)}>{t("invitations.none")}</p>;
  }
  return (
    <>
      {takenBack ? <Alert tone={alertTone.success}>{t("invitations.taken-back")}</Alert> : null}
      <ul {...partOf(part.list)} aria-label={t("invitations.open")}>
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
      {<ShowMoreOf label={t("invitations.more")} list={invitations} />}
    </>
  );
}
