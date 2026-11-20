import { reasonMax } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  Button,
  buttonLook,
  fieldKind,
  Form,
  PickerOption,
  SearchPicker,
  SubmitButton,
  TextField,
  Window,
  partOf,
  part,
} from "@bugaboo/ui";
import { useId, useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { pickerStatus, usePickerFrame } from "../../shared/picker-status.ts";
import { useWindowProps } from "../../shared/use-window-props.ts";
import { windowForms } from "../../shared/window-forms.ts";
import { linesOf, usePeople } from "../people/index.ts";
import { useAskSomeonesData } from "./data-exports.ts";

/** A person to choose, by name. */
interface Picked {
  readonly id: string;
  readonly name: string;
}

/** Whose data: the people the administrator sees by name, found by the words typed (one person picker, M5-F135). */
function PersonPicker({
  picked,
  onPick,
}: {
  readonly picked: Picked | undefined;
  readonly onPick: (person: Picked) => void;
}) {
  const { words, group, frame } = usePickerFrame();
  const found = usePeople(words);
  const people = linesOf(found.data?.pages);
  return (
    <SearchPicker
      legend={t("data-exports.pick")}
      searchLabel={t("data-exports.pick-search")}
      listLabel={t("data-exports.pick-list")}
      {...frame}
      {...pickerStatus(found, people.length, {
        nothing: t("data-exports.pick-nothing"),
        more: t("data-exports.pick-more"),
      })}
    >
      {people.map((person) => (
        <PickerOption
          key={person.id}
          group={group}
          checked={picked?.id === person.id}
          label={person.name}
          onPick={() => {
            onPick({ id: person.id, name: person.name });
          }}
        />
      ))}
    </SearchPicker>
  );
}

/** What the window asks: the person and the reason, until the server has taken the request. */
function useSomeonesDataForm(onClose: () => void) {
  const [picked, setPicked] = useState<Picked>();
  const [reason, setReason] = useState("");
  const [unpicked, setUnpicked] = useState(false);
  const ask = useAskSomeonesData();
  return {
    picked,
    reason,
    setReason,
    unpicked,
    ask,
    pick: (person: Picked): void => {
      setPicked(person);
      setUnpicked(false);
    },
    submit: (): void => {
      if (picked === undefined) {
        setUnpicked(true);
        return;
      }
      ask.mutate({ person: picked.id, reason: reason.trim() }, { onSuccess: onClose });
    },
  };
}

/**
 * *Download someone's data* (PRIV-03, ADR-0044, decision 4): a person picked by name and the reason, always; the
 * person is told, and the security log records who made the file of whose data and why. The window closes once the
 * server has taken the request, and the new file stands at the top of the list.
 */
export function SomeonesDataWindow({ onClose }: { readonly onClose: () => void }) {
  const formId = useId();
  const windowProps = useWindowProps(windowForms.someonesData);
  const form = useSomeonesDataForm(onClose);
  return (
    <Window
      {...windowProps}
      title={t("data-exports.someone")}
      isOpen
      onOpenChange={(isOpen) => {
        if (!isOpen) {
          onClose();
        }
      }}
      footer={
        <div {...partOf(part.windowFooterActions)}>
          <Button look={buttonLook.quiet} onPress={onClose}>
            {t("common.cancel")}
          </Button>
          <SubmitButton form={formId} isPending={form.ask.isPending}>
            {t("data-exports.create")}
          </SubmitButton>
        </div>
      }
    >
      <Form id={formId} onSubmit={form.submit}>
        <p {...partOf(part.text)}>{t("data-exports.window-intro")}</p>
        {form.ask.isError ? (
          <Alert tone={alertTone.danger}>{messageOf(form.ask.error)}</Alert>
        ) : null}
        <PersonPicker picked={form.picked} onPick={form.pick} />
        {form.unpicked ? (
          <Alert tone={alertTone.danger}>{t("data-exports.pick-first")}</Alert>
        ) : null}
        <TextField
          label={t("data-exports.reason")}
          description={t("data-exports.reason-help")}
          kind={fieldKind.multiline}
          requiredMessage={t("error.field.required")}
          maxLength={reasonMax}
          value={form.reason}
          onChange={form.setReason}
        />
      </Form>
    </Window>
  );
}
