import { t } from "@bugaboo/messages";
import { Alert, alertTone, CopyButton, Form, SubmitButton } from "@bugaboo/ui";
import { useState } from "react";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { EmailField } from "../../shared/fields.tsx";
import styles from "./People.module.css";
import { refreshInvitations } from "./invitations.ts";

export interface CreatedLinkProps {
  readonly email: string;
  /** The page the link opens, with the secret after the hash sign. */
  readonly linkPath: string;
}

/** The link of a new invitation, shown once: the server keeps only a hash of its secret. */
function CreatedLink({ email, linkPath }: CreatedLinkProps) {
  const link = `${window.location.origin}${linkPath}`;
  return (
    <div className={styles.stack}>
      <Alert tone={alertTone.success}>{t("invitations.created", { email })}</Alert>
      <p className={styles.muted}>{t("invitations.link-help")}</p>
      <code className={styles.link} aria-label={t("invitations.link")}>
        {link}
      </code>
      <div className={styles.row}>
        <CopyButton
          text={link}
          label={t("invitations.copy")}
          copiedLabel={t("invitations.copied")}
        />
      </div>
    </div>
  );
}

/** *Make an invitation*: the address of the person, and then the link to send them. */
export function CreateInvitation() {
  const [email, setEmail] = useState("");
  const create = api.query.useMutation("post", "/api/v1/auth/invitations", {
    onSuccess: () => {
      setEmail("");
      refreshInvitations();
    },
  });
  return (
    <>
      <Form
        onSubmit={() => {
          create.mutate({ body: { email: email.trim() } });
        }}
      >
        <EmailField value={email} onChange={setEmail} />
        {create.isError ? <Alert tone={alertTone.danger}>{messageOf(create.error)}</Alert> : null}
        <div className={styles.row}>
          <SubmitButton isPending={create.isPending}>{t("invitations.create")}</SubmitButton>
        </div>
      </Form>
      {create.data === undefined ? null : (
        <CreatedLink email={create.data.invitation.email} linkPath={create.data.link_path} />
      )}
    </>
  );
}
