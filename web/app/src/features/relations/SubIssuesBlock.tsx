import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  Button,
  buttonLook,
  Form,
  SubmitButton,
  TextField,
  partOf,
  part,
} from "@bugaboo/ui";
import { useId, useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { itemsOf } from "../../shared/paged.ts";
import { IssueRefLine, type ShownIssue } from "./IssueRefLine.tsx";
import { setParentOf, useIssueRelations, useSubIssues, type BlockProps } from "./relations.ts";

/** What adding or taking out a sub-issue did: the issue just added, and what went wrong. */
function useParentChange() {
  const [added, setAdded] = useState<string | null>(null);
  const [error, setError] = useState<unknown>(null);
  const [pending, setPending] = useState(false);
  const put = async (child: string, parent: string | null) => {
    setPending(true);
    setError(null);
    try {
      await setParentOf(child, parent);
      setAdded(parent === null ? null : child);
    } catch (failure) {
      setError(failure);
    } finally {
      setPending(false);
    }
  };
  return { added, error, pending, put };
}

/** *Add sub-issue*: the readable ID of an issue that then sits under this one, with *Undo*. */
function AddSubIssue({ issueKey }: { readonly issueKey: string }) {
  const [other, setOther] = useState("");
  const { added, error, pending, put } = useParentChange();
  return (
    <Form
      onSubmit={() => {
        const child = other.trim();
        setOther("");
        void put(child, issueKey);
      }}
    >
      <TextField
        label={t("sub-issues.field")}
        requiredMessage={t("validation.required")}
        value={other}
        onChange={setOther}
      />
      <div {...partOf(part.row)}>
        <SubmitButton isPending={pending}>{t("sub-issues.add-it")}</SubmitButton>
      </div>
      {error === null ? null : <Alert tone={alertTone.danger}>{messageOf(error)}</Alert>}
      {added === null ? null : (
        <Alert tone={alertTone.success}>
          <span>{t("sub-issues.added", { key: added })}</span>
          <Button
            look={buttonLook.quiet}
            onPress={() => {
              void put(added, null);
            }}
          >
            {t("sub-issues.undo")}
          </Button>
        </Alert>
      )}
    </Form>
  );
}

/** A button that takes an issue out from under its parent. */
function TakeOut({ child, label }: { readonly child: string; readonly label: string }) {
  return (
    <Button
      look={buttonLook.quiet}
      onPress={() => {
        void setParentOf(child, null);
      }}
    >
      {label}
    </Button>
  );
}

/** *Part of: Goal …*, the issue this one sits under, when the viewer may see it. */
function PartOf({
  above,
  issueKey,
  mayChange,
}: {
  readonly above: ShownIssue;
  readonly issueKey: string;
  readonly mayChange: boolean;
}) {
  return (
    <div {...partOf(part.row)}>
      <span>{t("sub-issues.part-of")}</span>
      <IssueRefLine issue={above}>
        {mayChange ? (
          <TakeOut child={issueKey} label={t("sub-issues.not-part-of", { key: above.key })} />
        ) : null}
      </IssueRefLine>
    </div>
  );
}

/** How far the sub-issues are, in a bar and in words. */
function Progress({ done, total }: { readonly done: number; readonly total: number }) {
  return (
    <div {...partOf(part.row)}>
      <progress
        {...partOf(part.progress)}
        aria-label={t("sub-issues.progress-label")}
        value={done}
        max={total}
      />
      <span>{t("sub-issues.progress", { done, total })}</span>
    </div>
  );
}

/** The parts the viewer may see, one line each. */
function Parts({
  parts,
  mayChange,
}: {
  readonly parts: readonly Schema<"IssueRef">[];
  readonly mayChange: boolean;
}) {
  if (parts.length === 0) {
    return <p {...partOf(part.muted)}>{t("sub-issues.none")}</p>;
  }
  return (
    <ul {...partOf(part.list)}>
      {parts.map((sub) => (
        <li key={sub.key} {...partOf(part.item)}>
          <IssueRefLine issue={sub}>
            {mayChange ? (
              <TakeOut child={sub.key} label={t("sub-issues.take-out", { key: sub.key })} />
            ) : null}
          </IssueRefLine>
        </li>
      ))}
    </ul>
  );
}

/**
 * *Sub-issues* (ISS-03): the issue this one sits under, how far its parts are (counting only the
 * ones the viewer may see), the parts, and *Add sub-issue*.
 */
export function SubIssuesBlock({ issueKey }: BlockProps) {
  const heading = useId();
  const header = useIssueRelations(issueKey);
  const list = useSubIssues(issueKey);
  const above = header.data?.part_of;
  const progress = header.data?.progress;
  const mayChange = header.data?.may_change === true;
  return (
    <section {...partOf(part.stack)} aria-labelledby={heading}>
      <h3 id={heading}>{t("sub-issues.title")}</h3>
      {header.isError ? <Alert tone={alertTone.danger}>{messageOf(header.error)}</Alert> : null}
      {above === undefined || above === null ? null : (
        <PartOf above={above} issueKey={issueKey} mayChange={mayChange} />
      )}
      {progress === undefined || progress === null ? null : (
        <Progress done={progress.done} total={progress.total} />
      )}
      <Parts parts={itemsOf(list.data?.pages)} mayChange={mayChange} />
      {list.hasNextPage ? (
        <Button
          isPending={list.isFetchingNextPage}
          onPress={() => {
            void list.fetchNextPage();
          }}
        >
          {t("sub-issues.more")}
        </Button>
      ) : null}
      {mayChange ? <AddSubIssue issueKey={issueKey} /> : null}
    </section>
  );
}
