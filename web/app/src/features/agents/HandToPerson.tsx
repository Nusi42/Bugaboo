import { t } from "@bugaboo/messages";
import { AskDialog, Button, Card, PickerOption, SearchPicker, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { pickerStatus, usePickerFrame } from "../../shared/picker-status.ts";
import { useMe } from "../../shared/use-me.ts";
import { peopleOf, useAssignablePeople } from "../issues/index.ts";
import { useHandAgent, useTakeBackOffer, type ListedAgent } from "./agents.ts";
import { ActionError } from "./ActionError.tsx";
import { projectKey } from "./project-choices.ts";
import { ProjectSelect } from "./ProjectSelect.tsx";

/** The people of a project that the person can name, found by name and read a page at a time, but the person: who can take the agent. */
function PersonSelect({
  project,
  value,
  onChange,
}: {
  readonly project: string;
  readonly value: string;
  readonly onChange: (person: string) => void;
}) {
  // The agent is the person's own already: only somebody else can take it.
  const me = useMe()?.id;
  const { words, group, frame } = usePickerFrame();
  const found = useAssignablePeople(project, words);
  const people = peopleOf(found.data?.pages).filter((person) => person.id !== me);
  return (
    <SearchPicker
      legend={t("agents.hand-person")}
      searchLabel={t("agents.hand-person-search")}
      listLabel={t("agents.hand-person-list")}
      {...frame}
      {...pickerStatus(found, people.length, {
        nothing: t("agents.hand-person-nothing"),
        more: t("agents.hand-person-more"),
      })}
    >
      {people.map((person) => (
        <PickerOption
          key={person.id}
          group={group}
          checked={person.id === value}
          label={person.name}
          onPick={() => {
            onChange(person.id);
          }}
        />
      ))}
    </SearchPicker>
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
      <div {...partOf(part.row)}>
        <Button
          isPending={takeBack.isPending}
          onPress={() => {
            takeBack.mutate({ params: { path: { agent } } });
          }}
        >
          {t("agents.offer-take-back")}
        </Button>
      </div>
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
      <div {...partOf(part.row)}>
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
      </div>
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
