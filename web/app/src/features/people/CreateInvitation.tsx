import { t } from "@bugaboo/messages";
import { Alert, alertTone, CopyButton, Form, SubmitButton, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { EmailField } from "../../shared/fields.tsx";
import { refreshInvitations } from "./invitations.ts";

export interface CreatedLinkProps {
  readonly email: string;
  /** The page the link opens, with the secret after the hash sign. */
  readonly linkPath: string;
}

/** Marks of the place of the address in the message, which the message has once. */
const WHERE = "@@address@@";

/**
 * The address with room to break after its `@` and its dots, so a narrow screen breaks it at those places and
 * never inside a word (M5 exit, O8). The text stays the same: a break point is no character.
 */
function Address({ email }: { readonly email: string }) {
  const pieces = email.split(/(?<=[@.])/u);
  return pieces.map((piece, at) => (
    <span key={pieces.slice(0, at + 1).join("")}>
      {piece}
      <wbr />
    </span>
  ));
}

/** The message that an invitation was made, with the address of the person where it belongs. */
function CreatedMessage({ email }: { readonly email: string }) {
  const [before, after] = t("invitations.created", { email: WHERE }).split(WHERE);
  return (
    <>
      {before}
      <Address email={email} />
      {after}
    </>
  );
}

/** The link of a new invitation, shown once: the server keeps only a hash of its secret. */
function CreatedLink({ email, linkPath }: CreatedLinkProps) {
  const link = `${window.location.origin}${linkPath}`;
  return (
    <div {...partOf(part.stack)}>
      <Alert tone={alertTone.success}>
        <CreatedMessage email={email} />
      </Alert>
      <p {...partOf(part.muted)}>{t("invitations.link-help")}</p>
      <code {...partOf(part.keyBox, { variants: ["small"] })} aria-label={t("invitations.link")}>
        {link}
      </code>
      <div {...partOf(part.row)}>
        <CopyButton
          text={link}
          label={t("invitations.copy")}
          copiedLabel={t("invitations.copied")}
        />
      </div>
    </div>
  );
}

/** *Create an invitation*: the address of the person, and then the link to send them. */
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
        <div {...partOf(part.row)}>
          <SubmitButton isPending={create.isPending}>{t("invitations.create")}</SubmitButton>
        </div>
      </Form>
      {create.data === undefined ? null : (
        <CreatedLink email={create.data.invitation.email} linkPath={create.data.link_path} />
      )}
    </>
  );
}
