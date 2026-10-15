import { t } from "@bugaboo/messages";
import { Button, buttonLook, SubmitButton, partOf, part } from "@bugaboo/ui";
import { useNavigate } from "@tanstack/react-router";

import { layoutsAddress } from "../../shared/paths.ts";
import { closeOverlay } from "../shell/index.ts";
import { canCreate } from "./new-issue-form.ts";
import type { NewIssue } from "./use-new-issue.ts";

export interface NewIssueActionsProps {
  readonly form: NewIssue;
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

/** The buttons under the form: cancel, *Create and start another*, *Create*, and *Customize this form*. */
export function NewIssueActions({ form, onMade, onClose }: NewIssueActionsProps) {
  const blocked = !canCreate(form.values) || form.project === undefined;
  return (
    <div {...partOf(part.issueSectionActions)}>
      {blocked ? <p {...partOf(part.muted)}>{t("issues.create-another-why")}</p> : null}
      <CustomizeThisForm form={form} />
      <Button look={buttonLook.quiet} onPress={onClose}>
        {t("common.cancel")}
      </Button>
      <Button
        isDisabled={blocked}
        isPending={form.pending}
        onPress={() => {
          form.create(true, onMade);
        }}
      >
        {t("issues.create-another")}
      </Button>
      <SubmitButton isPending={form.pending}>{t("issues.create")}</SubmitButton>
    </div>
  );
}
