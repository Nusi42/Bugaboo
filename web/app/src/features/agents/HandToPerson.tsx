import { t } from "@bugaboo/messages";
import { AskDialog, Button, Card, SelectField } from "@bugaboo/ui";
import { useState } from "react";

import { peopleOf, useAssignablePeople } from "../issues/index.ts";
import { useHandAgent, useTakeBackOffer, type ListedAgent } from "./agents.ts";
import { ActionError } from "./ActionError.tsx";
import { projectKey } from "./project-choices.ts";
import { ProjectSelect } from "./ProjectSelect.tsx";

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

/** The project and then a person of it: the receiver of the agent. */
function PersonChoice({
  value,
  onChange,
}: {
  readonly value: string;
  readonly onChange: (person: string) => void;
}) {
  const [project, setProject] = useState("");
  return (
    <>
      <ProjectSelect
        label={t("agents.hand-project")}
        description={t("agents.hand-project-help")}
        valueKey={projectKey.shortName}
        value={project}
        onChange={(next) => {
          setProject(next);
          onChange("");
        }}
      />
      {project === "" ? null : <PersonSelect project={project} value={value} onChange={onChange} />}
    </>
  );
}

/** The note that the agent is offered, and the button that takes the offer back. */
function OfferNote({ agent }: { readonly agent: string }) {
  const takeBack = useTakeBackOffer();
  return (
    <>
      <p>{t("agents.hand-waiting")}</p>
      <Button
        isPending={takeBack.isPending}
        onPress={() => {
          takeBack.mutate({ params: { path: { agent } } });
        }}
      >
        {t("agents.offer-take-back")}
      </Button>
      <ActionError error={takeBack.error} />
    </>
  );
}

/**
 * *Hand to another person* (UX §4.17): the other person has to accept the agent, and takes it with
 * its name; its keys are taken back and they make their own.
 */
export function HandToPerson({ agent }: { readonly agent: ListedAgent }) {
  const [open, setOpen] = useState(false);
  const [person, setPerson] = useState("");
  const hand = useHandAgent();
  return (
    <Card heading={t("agents.hand")} intro={t("agents.hand-intro")}>
      <Button
        isDisabled={agent.state === "stopped" || agent.offered}
        onPress={() => {
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
        <PersonChoice value={person} onChange={setPerson} />
      </AskDialog>
      <ActionError error={hand.error} />
      {agent.offered ? <OfferNote agent={agent.id} /> : null}
    </Card>
  );
}
