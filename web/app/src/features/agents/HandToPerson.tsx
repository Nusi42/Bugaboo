import { t } from "@bugaboo/messages";
import { Alert, alertTone, AskDialog, Button, Card, SelectField } from "@bugaboo/ui";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { peopleOf, useAssignablePeople } from "../issues/index.ts";
import { useHandAgent, useProjectChoices, type ListedAgent } from "./agents.ts";

/** The people of a project that the person can name, as the choices of the window. */
function PersonSelect({
  project,
  value,
  onChange,
}: {
  readonly project: string;
  readonly value: string;
  readonly onChange: (person: string) => void;
}) {
  const people = peopleOf(useAssignablePeople(project, "").data?.pages);
  return (
    <SelectField
      label={t("agents.hand-person")}
      value={value}
      onChange={onChange}
      choices={[
        { value: "", label: t("agents.rule-choose") },
        ...people.map((person) => ({ value: person.id, label: person.name })),
      ]}
    />
  );
}

/** The project whose people are offered. */
function ProjectSelect({
  value,
  onChange,
}: {
  readonly value: string;
  readonly onChange: (project: string) => void;
}) {
  const projects = useProjectChoices().data?.items ?? [];
  const choices = [{ value: "", label: t("agents.rule-choose") }];
  for (const one of projects) {
    if (one.short_name !== undefined && one.short_name !== null) {
      choices.push({ value: one.short_name, label: one.name ?? one.short_name });
    }
  }
  return (
    <SelectField
      label={t("agents.hand-project")}
      description={t("agents.hand-project-help")}
      value={value}
      onChange={onChange}
      choices={choices}
    />
  );
}

/**
 * *Hand to another person* (UX §4.17): the other person has to accept the agent, and takes it with
 * its name; its keys are taken back and they make their own.
 */
export function HandToPerson({ agent }: { readonly agent: ListedAgent }) {
  const [open, setOpen] = useState(false);
  const [project, setProject] = useState("");
  const [person, setPerson] = useState("");
  const hand = useHandAgent();
  return (
    <Card heading={t("agents.hand")} intro={t("agents.hand-intro")}>
      <Button
        isDisabled={agent.state === "stopped" || agent.offered}
        onPress={() => {
          setProject("");
          setPerson("");
          hand.reset();
          setOpen(true);
        }}
      >
        {t("agents.hand")}
      </Button>
      <AskDialog
        isOpen={open}
        onOpenChange={setOpen}
        title={t("agents.hand-title", { agent: agent.name })}
        text={t("agents.hand-text")}
        confirmLabel={t("agents.hand-confirm")}
        cancelLabel={t("common.cancel")}
        confirmDisabled={person === "" || hand.isPending}
        confirmHint={t("agents.hand-need")}
        onConfirm={() => {
          hand.mutate({
            params: { path: { agent: agent.id } },
            body: { to: person, version: agent.version },
          });
        }}
      >
        <ProjectSelect
          value={project}
          onChange={(next) => {
            setProject(next);
            setPerson("");
          }}
        />
        {project === "" ? null : (
          <PersonSelect project={project} value={person} onChange={setPerson} />
        )}
      </AskDialog>
      {hand.isError ? <Alert tone={alertTone.danger}>{messageOf(hand.error)}</Alert> : null}
      {agent.offered ? <p>{t("agents.hand-waiting")}</p> : null}
    </Card>
  );
}
