import { t } from "@bugaboo/messages";
import { Checkbox, ChoiceGroup, choiceGroupLook, partOf, part } from "@bugaboo/ui";

import { HandOverPowers } from "../agent-work/index.ts";
import { projectKey } from "./project-choices.ts";
import { ProjectSelect } from "./ProjectSelect.tsx";
import { type DraftProps } from "./rule-draft.ts";

/** The powers of the hand-overs of a rule that hands over, and whether they may be accepted by themselves. */
function HandOverFields({ draft, onChange }: DraftProps) {
  return (
    <div {...partOf(part.stack)}>
      <p {...partOf(part.muted)}>{t("agents.rule-hand-over-help")}</p>
      <HandOverPowers
        draft={draft.handOver}
        onChange={(change) => {
          onChange({ handOver: { ...draft.handOver, ...change } });
        }}
      />
      <Checkbox
        label={t("agents.rule-accept-by-itself")}
        isSelected={draft.handOver.acceptByItself}
        onChange={(on) => {
          onChange({ handOver: { ...draft.handOver, acceptByItself: on } });
        }}
      />
      <p {...partOf(part.muted)}>{t("agents.rule-accept-by-itself-help")}</p>
    </div>
  );
}

/** What the rule does with what it finds: it proposes, or it hands each issue over; and it may create issues in a project. */
export function RuleWayFields({ draft, onChange }: DraftProps) {
  return (
    <div {...partOf(part.stack)}>
      <ChoiceGroup
        legend={t("agents.rule-may")}
        look={choiceGroupLook.radios}
        choices={[
          { value: "proposals", label: t("agents.rule-propose") },
          { value: "hand-over", label: t("agents.rule-hand-over") },
        ]}
        value={draft.way}
        onChange={(way) => {
          onChange({ way });
        }}
      />
      {draft.way === "hand-over" ? <HandOverFields draft={draft} onChange={onChange} /> : null}
      <p {...partOf(part.muted)}>{t("agents.rule-never")}</p>
      <Checkbox
        label={t("agents.rule-creates")}
        isSelected={draft.createIssues}
        onChange={(on) => {
          onChange({ createIssues: on });
        }}
      />
      {draft.createIssues ? (
        <ProjectSelect
          label={t("agents.rule-creates-in")}
          valueKey={projectKey.id}
          value={draft.createsIn}
          onChange={(createsIn) => {
            onChange({ createsIn });
          }}
        />
      ) : null}
    </div>
  );
}
