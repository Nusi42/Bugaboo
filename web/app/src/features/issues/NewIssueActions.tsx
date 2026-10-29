import { t } from "@bugaboo/messages";
import { Button, buttonLook, SubmitButton, partOf, part, useLayout } from "@bugaboo/ui";
import { useNavigate } from "@tanstack/react-router";
import { useId } from "react";

import { layoutsAddress } from "../../shared/paths.ts";
import { closeOverlay } from "../shell/index.ts";
import { canCreate } from "./new-issue-form.ts";
import type { NewIssue } from "./use-new-issue.ts";

export interface NewIssueActionsProps {
  readonly form: NewIssue;
  /** The form that *Create* sends: the buttons stand in the footer of the window, outside it. */
  readonly formId: string;
  readonly onMade: (key: string, project: string) => void;
  readonly onClose: () => void;
}

/** *Customize this form*, for people who may change the layout of the project: opens its editor for this type. */
function CustomizeThisForm({ form }: { readonly form: NewIssue }) {
  const navigate = useNavigate();
  const shortName = form.project?.shortName;
  if (!form.canCustomize || shortName === undefined) {
    return null;
  }
  return (
    <Button
      look={buttonLook.quiet}
      onPress={() => {
        closeOverlay();
        void navigate({
          href: layoutsAddress(shortName, {
            form: "new-issue",
            type: form.values.issueType === "" ? undefined : form.values.issueType,
          }),
        });
      }}
    >
      {t("layouts.customize-form")}
    </Button>
  );
}

/** What the form still lacks before it can be sent, in words; nothing when it can be sent. */
function missingForCreate(form: NewIssue) {
  if (form.project === undefined) {
    return "issues.missing-project";
  }
  if (form.values.title.trim() === "") {
    return "issues.missing-title";
  }
  // A project that offers no type says so by the type; the footer does not ask for one as well.
  return form.noType || canCreate(form.values) ? undefined : "issues.missing-type";
}

/**
 * The footer of the new-issue window (UX §4.5, §6.3): what is missing (it comes and goes with the
 * title), the buttons at the right with the main one last (R6), and below them the note: that the
 * draft is kept, and *Customize this form*. The note under the buttons never changes while the
 * person types, so the buttons stay where a finger is on its way to them. The footer stays in place
 * while the form scrolls.
 */
export function NewIssueActions({ form, formId, onMade, onClose }: NewIssueActionsProps) {
  const why = useId();
  const missing = missingForCreate(form);
  const phone = useLayout() === "phone";
  return (
    <>
      <p id={why} {...partOf(part.muted)}>
        {t(missing ?? "issues.ready-to-create")}
      </p>
      <div {...partOf(part.windowFooterActions)}>
        {phone ? null : (
          <>
            <Button look={buttonLook.quiet} onPress={onClose}>
              {t("common.cancel")}
            </Button>
            <CreateAnother form={form} missing={missing} why={why} onMade={onMade} />
          </>
        )}
        <SubmitButton form={formId} isPending={form.pending} isDisabled={missing !== undefined}>
          {t("issues.create")}
        </SubmitButton>
      </div>
      {phone ? null : <DraftNote form={form} />}
    </>
  );
}

/** *Create and start another*, which keeps the project and the type. */
function CreateAnother({
  form,
  missing,
  why,
  onMade,
}: {
  readonly form: NewIssue;
  readonly missing: string | undefined;
  readonly why: string;
  readonly onMade: NewIssueActionsProps["onMade"];
}) {
  return (
    <Button
      isDisabled={missing !== undefined}
      {...(missing === undefined ? {} : { "aria-describedby": why })}
      isPending={form.pending}
      onPress={() => {
        form.create(true, onMade);
      }}
    >
      {t("issues.create-another")}
    </Button>
  );
}

/** That the draft is kept, and *Customize this form*. */
function DraftNote({ form }: { readonly form: NewIssue }) {
  return (
    <div {...partOf(part.windowFooterNote)}>
      <p {...partOf(part.muted)}>{t("issues.draft-kept")}</p>
      <CustomizeThisForm form={form} />
    </div>
  );
}

/**
 * What a phone keeps out of the fixed footer, which holds the hint and the main button only: the other
 * answers and the note stand at the end of the form, in the part that scrolls (M5 exit, O13).
 */
export function NewIssueMoreActions({
  form,
  onMade,
  onClose,
}: Omit<NewIssueActionsProps, "formId">) {
  const why = useId();
  const missing = missingForCreate(form);
  return (
    <div {...partOf(part.stack)}>
      <CreateAnother form={form} missing={missing} why={why} onMade={onMade} />
      <Button look={buttonLook.quiet} onPress={onClose}>
        {t("common.cancel")}
      </Button>
      <DraftNote form={form} />
    </div>
  );
}
