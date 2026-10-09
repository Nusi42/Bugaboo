import { t } from "@bugaboo/messages";
import { Alert, alertTone, Switch, partOf, part } from "@bugaboo/ui";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { refreshIssueTypes } from "../../shared/query-keys.ts";
import type { IssueTypeLine } from "./issue-types.ts";

/**
 * *Retire*: no new issues of the type, while the old ones keep it (ISS-02). A switch, because it
 * can be taken back at once.
 */
export function RetireIssueType({ issueType }: { readonly issueType: IssueTypeLine }) {
  const change = api.query.useMutation("patch", "/api/v1/issue-types/{issue_type}", {
    onSuccess: refreshIssueTypes,
  });
  return (
    <div {...partOf(part.stack)}>
      <p {...partOf(part.muted)}>{t("issue-types.retire-help")}</p>
      <Switch
        label={issueType.retired ? t("issue-types.offer-again") : t("issue-types.retire")}
        isSelected={issueType.retired}
        onChange={(retired) => {
          change.mutate({
            params: { path: { issue_type: issueType.id } },
            body: { version: issueType.version, retired },
          });
        }}
      />
      {change.isError ? <Alert tone={alertTone.danger}>{messageOf(change.error)}</Alert> : null}
    </div>
  );
}
