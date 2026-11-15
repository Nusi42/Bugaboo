import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  Button,
  buttonLook,
  doNothing,
  Form,
  SubmitButton,
  TextField,
  partOf,
  part,
} from "@bugaboo/ui";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { useRelationChange } from "./relation-change.ts";
import { IssueRefLine } from "./IssueRefLine.tsx";
import { setParentOf } from "./relations.ts";
import type { ShownIssue } from "./shown-issue.ts";

// The changes of the parent of an issue in the section *Sub-issues* (`SubIssuesBlock.tsx`): *Add
// sub-issue*, *Part of* with the way to take the issue out, and taking a part out.

/** What adding or taking out a sub-issue did: the issue just added, and what went wrong. */
function useParentChange() {
  const [added, setAdded] = useState<string | null>(null);
  const change = useRelationChange(
    (put: { readonly child: string; readonly parent: string | null }) =>
      setParentOf(put.child, put.parent),
  );
  /**
   * Says whether the change was made; `made` changes the screen with the sentence, in the same turn, so both show in
   * one frame.
   */
  const put = async (
    child: string,
    parent: string | null,
    made: () => void = doNothing,
  ): Promise<boolean> => {
    try {
      await change.mutateAsync({ child, parent });
      made();
      setAdded(parent === null ? null : child);
      return true;
    } catch {
      return false;
    }
  };
  return { added, error: change.error, pending: change.isPending, put };
}

/** The small form of *Add sub-issue*: the readable ID of an issue that then sits under this one. */
function AddSubIssueForm({
  pending,
  onAdd,
  onCancel,
}: {
  readonly pending: boolean;
  /** Adds the issue; says whether it was added. */
  readonly onAdd: (child: string) => Promise<boolean>;
  readonly onCancel: () => void;
}) {
  const [other, setOther] = useState("");
  return (
    <Form
      onSubmit={() => {
        // The ID stays in the box until the issue is added: a refusal leaves it to be fixed.
        void onAdd(other.trim()).then((added) => {
          if (added) {
            setOther("");
          }
        });
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
export function AddSubIssue({ readableId }: { readonly readableId: string }) {
  const [open, setOpen] = useState(false);
  const { added, error, pending, put } = useParentChange();
  return (
    <>
      {open ? (
        <AddSubIssueForm
          pending={pending}
          onAdd={(child) => {
            // The form stays until the issue stands in the list: it goes in the same frame as
            // the sentence comes, so the section changes once (UX §6.4).
            return put(child, readableId, () => {
              setOpen(false);
            });
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
export function TakeOut({ child, label }: { readonly child: string; readonly label: string }) {
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
export function PartOf({
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
            label={t("sub-issues.not-part-of", { key: readableId, parent: above.readable_id })}
          />
        ) : null}
      </IssueRefLine>
    </div>
  );
}
