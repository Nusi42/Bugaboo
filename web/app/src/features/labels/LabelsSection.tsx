import { t } from "@bugaboo/messages";
import { Card, LabelMark, partOf, part } from "@bugaboo/ui";

import { ListEditor, type ListRow } from "../shell/index.ts";
import { EditLabel } from "./EditLabel.tsx";
import type { Label, LabelGroup } from "./labels.ts";
import { CreateLabelForm } from "./CreateLabelForm.tsx";
import type { OpenProps } from "./OpenProps.ts";

interface LabelsSectionProps extends OpenProps {
  readonly labels: readonly Label[];
  readonly groups: readonly LabelGroup[];
}

/** The row of a label: its group beside its name. */
function rowOf(label: Label, props: LabelsSectionProps): ListRow {
  const group = props.groups.find((each) => each.id === label.label_group);
  return {
    key: label.id,
    name: label.name,
    lead: <LabelMark color={label.color} />,
    details: (
      <span {...partOf(part.muted)}>
        {group?.name ?? t("labels.no-label-group")}
        {label.retired ? ` ${t("labels.retired")}` : null}
      </span>
    ),
    editor: <EditLabel key={label.id} label={label} groups={props.groups} others={props.labels} />,
  };
}

/** The labels as a list that opens next to it, and the form that makes one. */
export function LabelsSection(props: LabelsSectionProps) {
  return (
    <Card heading={t("labels.labels-title")}>
      <ListEditor
        rows={props.labels.map((label) => rowOf(label, props))}
        searchLabel={t("labels.labels-search")}
        nothing={t("labels.labels-nothing")}
        none={t("labels.labels-none-yet")}
        openKey={props.open}
        onOpen={props.onOpen}
        onClose={props.onClose}
      />
      <h3 {...partOf(part.cardTopic)}>{t("labels.create-label-title")}</h3>
      <p {...partOf(part.cardIntro)}>{t("labels.create-label-intro")}</p>
      <CreateLabelForm groups={props.groups} />
    </Card>
  );
}
