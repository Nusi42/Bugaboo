import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, Card, IssueTypeIcon, partOf, part } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { ListEditor, type ListRow } from "../shell/index.ts";
import { EditIssueType } from "./EditIssueType.tsx";
import { issueTypeLinesOf, useIssueTypes, type IssueTypeLine } from "./issue-types.ts";
import { MakeIssueTypeForm } from "./MakeIssueTypeForm.tsx";
import { RetireIssueType } from "./RetireIssueType.tsx";

export interface IssueTypesPageProps {
  /** The ID of the issue type that is open in the side panel, from the address. */
  readonly open: string | undefined;
  readonly onOpen: (id: string) => void;
  readonly onClose: () => void;
}

/** The row of an issue type: its icon and, while it is retired, a word that says so. */
function rowOf(issueType: IssueTypeLine): ListRow {
  return {
    key: issueType.id,
    name: issueType.name,
    details: (
      <>
        <IssueTypeIcon name={issueType.icon} />
        {issueType.retired ? <span {...partOf(part.muted)}>{t("issue-types.retired")}</span> : null}
      </>
    ),
    editor: (
      <div {...partOf(part.stack)}>
        <EditIssueType key={issueType.id} issueType={issueType} />
        <RetireIssueType issueType={issueType} />
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
      <Card heading={t("nav.issue-types")} level={1}>
        <p {...partOf(part.text)}>{t("issue-types.intro")}</p>
        {list.isError ? <Alert tone={alertTone.danger}>{messageOf(list.error)}</Alert> : null}
        <ListEditor
          rows={rows}
          searchLabel={t("issue-types.search")}
          nothing={t("issue-types.nothing")}
          openKey={open}
          onOpen={onOpen}
          onClose={onClose}
        />
        {list.hasNextPage ? (
          <Button
            isPending={list.isFetchingNextPage}
            onPress={() => {
              void list.fetchNextPage();
            }}
          >
            {t("issue-types.more")}
          </Button>
        ) : null}
      </Card>
      <Card heading={t("issue-types.make-title")}>
        <p {...partOf(part.text)}>{t("issue-types.make-intro")}</p>
        <MakeIssueTypeForm />
      </Card>
    </div>
  );
}
