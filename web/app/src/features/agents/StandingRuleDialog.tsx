import { agentRuleDaysMost } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  AskDialog,
  Checkbox,
  ChoiceGroup,
  SelectField,
  TextField,
  partOf,
  part,
} from "@bugaboo/ui";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { SavedSearchChoice } from "../search/index.ts";
import { listedAgents, useCreateRule, useMyAgentList, useProjectChoices } from "./agents.ts";
import {
  emptyRuleDraft,
  offeredFields,
  ruleBody,
  ruleFieldText,
  ruleIsComplete,
  withField,
  daysAreRight,
  type DraftProps,
  type RuleDraft,
} from "./rule-draft.ts";

/** The agents of the person that can work, as the choices of the window. */
function AgentSelect({ draft, onChange }: DraftProps) {
  const agents = listedAgents(useMyAgentList().data?.items).filter(
    (agent) => agent.state === "active",
  );
  return (
    <SelectField
      label={t("agents.rule-agent")}
      value={draft.agent}
      onChange={(agent) => {
        onChange({ agent });
      }}
      choices={[
        { value: "", label: t("agents.rule-choose") },
        ...agents.map((agent) => ({ value: agent.id, label: agent.name })),
      ]}
    />
  );
}

/** The projects the person can give agents work in. */
function ProjectSelect({
  label,
  value,
  onChange,
}: {
  readonly label: string;
  readonly value: string;
  readonly onChange: (project: string) => void;
}) {
  const projects = useProjectChoices().data?.items ?? [];
  return (
    <SelectField
      label={label}
      value={value}
      onChange={onChange}
      choices={[
        { value: "", label: t("agents.rule-choose") },
        ...projects.flatMap((project) =>
          project.id === undefined || project.id === null
            ? []
            : [{ value: project.id, label: project.name ?? project.short_name ?? project.id }],
        ),
      ]}
    />
  );
}

/** What the rule may do: only propose, or change the fields of the short list, and create issues in a project. */
function Powers({ draft, onChange }: DraftProps) {
  return (
    <div {...partOf(part.stack)}>
      <ChoiceGroup
        legend={t("agents.rule-may")}
        choices={[
          { value: "propose", label: t("agents.rule-propose") },
          { value: "change", label: t("agents.rule-change") },
        ]}
        value={draft.power}
        onChange={(power) => {
          onChange({ power });
        }}
      />
      {draft.power === "change" ? (
        <fieldset {...partOf(part.agentWorkGroup)}>
          <legend {...partOf(part.agentWorkGroupName)}>{t("agents.rule-fields")}</legend>
          {offeredFields.map((field) => (
            <Checkbox
              key={field}
              label={ruleFieldText(field)}
              isSelected={draft.fields.includes(field)}
              onChange={(on) => {
                onChange({ fields: withField(draft.fields, field, on) });
              }}
            />
          ))}
        </fieldset>
      ) : null}
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
          value={draft.createsIn}
          onChange={(createsIn) => {
            onChange({ createsIn });
          }}
        />
      ) : null}
    </div>
  );
}

/** The fields of the window. */
function Fields({ draft, onChange, searchFixed }: DraftProps & { readonly searchFixed: boolean }) {
  return (
    <div {...partOf(part.stack)}>
      {searchFixed ? null : (
        <SavedSearchChoice
          label={t("agents.rule-search")}
          value={draft.search}
          onChange={(search) => {
            onChange({ search });
          }}
        />
      )}
      <AgentSelect draft={draft} onChange={onChange} />
      <ProjectSelect
        label={t("agents.rule-project")}
        value={draft.project}
        onChange={(project) => {
          onChange({ project });
        }}
      />
      <Powers draft={draft} onChange={onChange} />
      <TextField
        label={t("agents.rule-days")}
        description={t("agents.rule-days-help", { most: agentRuleDaysMost })}
        errorMessage={
          daysAreRight(draft.days)
            ? undefined
            : t("agents.rule-days-wrong", { most: agentRuleDaysMost })
        }
        value={draft.days}
        onChange={(days) => {
          onChange({ days });
        }}
      />
    </div>
  );
}

export interface StandingRuleDialogProps {
  readonly isOpen: boolean;
  readonly onOpenChange: (open: boolean) => void;
  /** Where the window starts: the saved search, the agent. */
  readonly start?: Partial<RuleDraft>;
}

/** The window starts empty. */
const noStart: Partial<RuleDraft> = {};

/**
 * The window of a standing rule (UX §4.17, §6.3): the saved search, the agent, what it may do (only
 * propose, or change the fields of a short list, never people or the status; or create issues in a
 * project), and when it runs out (30 days, at most 90).
 */
export function StandingRuleDialog({
  isOpen,
  onOpenChange,
  start = noStart,
}: StandingRuleDialogProps) {
  const [draft, setDraft] = useState<RuleDraft>(() => emptyRuleDraft(start));
  const create = useCreateRule();
  return (
    <>
      <AskDialog
        isOpen={isOpen}
        onOpenChange={onOpenChange}
        title={t("agents.rule-title")}
        text={t("agents.rule-text")}
        confirmLabel={t("agents.rule-confirm")}
        cancelLabel={t("common.cancel")}
        confirmDisabled={!ruleIsComplete(draft) || create.isPending}
        onConfirm={() => {
          create.mutate({ body: ruleBody(draft) });
        }}
      >
        <Fields
          draft={draft}
          searchFixed={start.search !== undefined}
          onChange={(change) => {
            setDraft((was) => ({ ...was, ...change }));
          }}
        />
      </AskDialog>
      {create.isError ? <Alert tone={alertTone.danger}>{messageOf(create.error)}</Alert> : null}
    </>
  );
}
