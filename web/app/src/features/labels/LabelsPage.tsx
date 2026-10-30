import { t } from "@bugaboo/messages";
import { Alert, alertTone, Card, cardLook, partOf, part } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { GroupsSection } from "./GroupsSection.tsx";
import { useLabels } from "./labels.ts";
import { LabelsSection } from "./LabelsSection.tsx";
import type { OpenProps } from "./OpenProps.ts";
import { TagsSection } from "./TagsSection.tsx";

export type LabelsPageProps = OpenProps;

/**
 * *Organization settings › Labels and tags* (TAG-02, TAG-03): the groups of labels, the labels and
 * the tags as searchable lists, each opening next to it to change it, rename it (with a preview of
 * what it does to the issues), combine it into another, or retire it; and the forms that make a
 * group and a label.
 */
export function LabelsPage(props: LabelsPageProps) {
  usePageTitle(t("nav.labels"));
  const labels = useLabels();
  return (
    <div {...partOf(part.stack)}>
      <Card heading={t("nav.labels")} level={1} look={cardLook.pageHead} intro={t("labels.intro")}>
        {labels.isError ? <Alert tone={alertTone.danger}>{messageOf(labels.error)}</Alert> : null}
      </Card>
      <GroupsSection groups={labels.groups} {...props} />
      <LabelsSection labels={labels.labels} groups={labels.groups} {...props} />
      <TagsSection {...props} />
    </div>
  );
}
