import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, Card, partOf, part } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { ListEditor, type ListRow } from "../shell/index.ts";
import { EditRelationKind } from "./EditRelationKind.tsx";
import { MakeRelationKindForm } from "./MakeRelationKindForm.tsx";
import { relationKindLinesOf, useRelationKinds, type RelationKindLine } from "./relation-kinds.ts";

export interface RelationKindsPageProps {
  /** The ID of the kind that is open in the side panel, from the address. */
  readonly open: string | undefined;
  readonly onOpen: (id: string) => void;
  readonly onClose: () => void;
}

/** The row of a kind: both ends in one name, and a word that says when it is retired. */
function rowOf(kind: RelationKindLine): ListRow {
  return {
    key: kind.id,
    name: t("relation-kinds.reads", { forward: kind.forward, backward: kind.backward }),
    details: kind.retired ? (
      <span {...partOf(part.muted)}>{t("relation-kinds.retired")}</span>
    ) : null,
    editor: <EditRelationKind key={kind.id} kind={kind} />,
  };
}

/**
 * *Organization settings › Kinds of relation* (ISS-04): the kinds as a searchable list, where one
 * opens next to it to change its words or to retire it; and the form that makes a kind.
 */
export function RelationKindsPage({ open, onOpen, onClose }: RelationKindsPageProps) {
  usePageTitle(t("nav.relation-kinds"));
  const list = useRelationKinds();
  const rows = relationKindLinesOf(list.data?.pages).map((kind) => rowOf(kind));
  return (
    <div {...partOf(part.stack)}>
      <Card heading={t("nav.relation-kinds")} level={1}>
        <p {...partOf(part.text)}>{t("relation-kinds.intro")}</p>
        {list.isError ? <Alert tone={alertTone.danger}>{messageOf(list.error)}</Alert> : null}
        <ListEditor
          rows={rows}
          searchLabel={t("relation-kinds.search")}
          nothing={t("relation-kinds.nothing")}
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
            {t("relation-kinds.more")}
          </Button>
        ) : null}
      </Card>
      <Card heading={t("relation-kinds.make-title")}>
        <p {...partOf(part.text)}>{t("relation-kinds.make-intro")}</p>
        <MakeRelationKindForm />
      </Card>
    </div>
  );
}
