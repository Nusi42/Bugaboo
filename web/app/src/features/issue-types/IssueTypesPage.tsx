import { t } from "@bugaboo/messages";
import { Alert, alertTone, Card, IssueTypeIcon, partOf, part, ShowMoreOf } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { ListEditor, type ListRow } from "../shell/index.ts";
import { EditIssueType } from "./EditIssueType.tsx";
import { IssueTypeOffers } from "./IssueTypeOffers.tsx";
import { issueTypeLinesOf, useIssueTypes, type IssueTypeLine } from "./issue-types.ts";
import { CreateIssueTypeForm } from "./CreateIssueTypeForm.tsx";
import { RetireIssueType } from "./RetireIssueType.tsx";

export interface IssueTypesPageProps {
  /** The ID of the issue type that is open in the side panel, from the address. */
  readonly open: string | undefined;
  readonly onOpen: (id: string) => void;
  readonly onClose: () => void;
}

/** The row of an issue type: its icon before the name and, while it is retired, a word that says so. */
function rowOf(issueType: IssueTypeLine): ListRow {
  return {
    key: issueType.id,
    name: issueType.name,
    lead: <IssueTypeIcon name={issueType.icon} />,
    details: issueType.retired ? (
      <span {...partOf(part.muted)}>{t("issue-types.retired")}</span>
    ) : null,
    editor: (
      <div {...partOf(part.stack)}>
        {issueType.mayManage ? (
          <>
            <EditIssueType key={issueType.id} issueType={issueType} />
            <RetireIssueType issueType={issueType} />
          </>
        ) : null}
        <IssueTypeOffers key={issueType.id} issueType={issueType} />
      </div>
    ),
  };
}

/**
 * *Organization settings › Issue types* (ISS-02): the types as a searchable list, where one opens
 * next to it to change its name and icon or to retire it; and the form that makes a type.
 */
export function IssueTypesPage({ open, onOpen, onClose }: IssueTypesPageProps) {
  usePageTitle(t("nav.issue-types"));
  const list = useIssueTypes();
  const rows = issueTypeLinesOf(list.data?.pages).map((issueType) => rowOf(issueType));
  return (
    <div {...partOf(part.stack)}>
      <Card heading={t("nav.issue-types")} level={1} intro={t("issue-types.intro")}>
        {list.isError ? <Alert tone={alertTone.danger}>{messageOf(list.error)}</Alert> : null}
        <ListEditor
          rows={rows}
          searchLabel={t("issue-types.search")}
          nothing={t("issue-types.nothing")}
          openKey={open}
          onOpen={onOpen}
          onClose={onClose}
        />
        <ShowMoreOf label={t("issue-types.more")} list={list} />
      </Card>
      <Card heading={t("issue-types.create-title")} intro={t("issue-types.create-intro")}>
        <CreateIssueTypeForm />
      </Card>
    </div>
  );
}
