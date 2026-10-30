import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  Button,
  buttonLook,
  Form,
  ListGrid,
  SubmitButton,
  TextField,
  partOf,
  part,
  ShowMoreOf,
} from "@bugaboo/ui";
import { useId, useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { itemsOf } from "../../shared/paged.ts";
import { IssueRefLine } from "./IssueRefLine.tsx";
import type { ShownIssue } from "./shown-issue.ts";
import { IssueRefRow } from "./IssueRefRow.tsx";
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

/** The small form of *Add sub-issue*: the readable ID of an issue that then sits under this one. */
function AddSubIssueForm({
  pending,
  onAdd,
  onCancel,
}: {
  readonly pending: boolean;
  readonly onAdd: (child: string) => void;
  readonly onCancel: () => void;
}) {
  const [other, setOther] = useState("");
  return (
    <Form
      onSubmit={() => {
        onAdd(other.trim());
        setOther("");
      }}
    >
      <TextField
        label={t("common.issue-id")}
        requiredMessage={t("error.field.required")}
        value={other}
        onChange={setOther}
      />
      <div {...partOf(part.row)}>
        <SubmitButton isPending={pending}>{t("sub-issues.add-it")}</SubmitButton>
        <Button look={buttonLook.quiet} onPress={onCancel}>
          {t("common.cancel")}
        </Button>
      </div>
    </Form>
  );
}

/**
 * *Add sub-issue* (UX §4.4: *Sub-issues · None yet · Add sub-issue*): a button that opens the small
 * form, and after adding, the issue that was added with *Undo*.
 */
function AddSubIssue({ readableId }: { readonly readableId: string }) {
  const [open, setOpen] = useState(false);
  const { added, error, pending, put } = useParentChange();
  return (
    <>
      {open ? (
        <AddSubIssueForm
          pending={pending}
          onAdd={(child) => {
            setOpen(false);
            void put(child, readableId);
          }}
          onCancel={() => {
            setOpen(false);
          }}
        />
      ) : (
        <div {...partOf(part.row)}>
          <Button
            onPress={() => {
              setOpen(true);
            }}
          >
            {t("sub-issues.add")}
          </Button>
        </div>
      )}
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
            {t("common.undo")}
          </Button>
        </Alert>
      )}
    </>
  );
}

/** A button that takes an issue out from under its parent. */
function TakeOut({ child, label }: { readonly child: string; readonly label: string }) {
  return (
    <Button
      look={buttonLook.quiet}
      aria-label={label}
      onPress={() => {
        void setParentOf(child, null);
      }}
    >
      {t("sub-issues.take-out-short", { key: child })}
    </Button>
  );
}

/** *Part of: Goal …*, the issue this one sits under, when the viewer may see it. */
function PartOf({
  above,
  readableId,
  mayChange,
}: {
  readonly above: ShownIssue;
  readonly readableId: string;
  readonly mayChange: boolean;
}) {
  return (
    <div {...partOf(part.row)}>
      <span>{t("issue-field.parent")}</span>
      <IssueRefLine issue={above}>
        {mayChange ? (
          <TakeOut
            child={readableId}
            label={t("sub-issues.not-part-of", { key: above.readable_id })}
          />
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
    <ListGrid label={t("sub-issues.title")}>
      {parts.map((sub) => (
        <IssueRefRow
          key={sub.readable_id}
          issue={sub}
          action={
            mayChange ? (
              <TakeOut
                child={sub.readable_id}
                label={t("sub-issues.take-out", { key: sub.readable_id })}
              />
            ) : undefined
          }
        />
      ))}
    </ListGrid>
  );
}

/**
 * *Sub-issues* (ISS-03): the issue this one sits under, how far its parts are (counting only the
 * ones the viewer may see), the parts, and *Add sub-issue*.
 */
export function SubIssuesBlock({ readableId }: BlockProps) {
  const heading = useId();
  const header = useIssueRelations(readableId);
  const list = useSubIssues(readableId);
  const above = header.data?.part_of;
  const progress = header.data?.progress;
  const mayChange = header.data?.may_change === true;
  return (
    <section {...partOf(part.stack)} aria-labelledby={heading}>
      <h2 id={heading} {...partOf(part.issueSectionTitle)}>
        {t("sub-issues.title")}
      </h2>
      {header.isError ? <Alert tone={alertTone.danger}>{messageOf(header.error)}</Alert> : null}
      {above === undefined || above === null ? null : (
        <PartOf above={above} readableId={readableId} mayChange={mayChange} />
      )}
      {progress === undefined || progress === null ? null : (
        <Progress done={progress.done} total={progress.total} />
      )}
      <Parts parts={itemsOf(list.data?.pages)} mayChange={mayChange} />
      <ShowMoreOf label={t("sub-issues.more")} list={list} />
      {mayChange ? <AddSubIssue readableId={readableId} /> : null}
    </section>
  );
}
