import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  Button,
  Form,
  PickerRow,
  SearchPicker,
  SubmitButton,
  TextField,
} from "@bugaboo/ui";
import { useState } from "react";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { pickerStatus, usePickerFrame } from "../../shared/picker-status.ts";
import { groupLinesOf, useGroups } from "../people/index.ts";
import { ChoiceDialog } from "./ChoiceDialog.tsx";
import type { OwnField } from "./fields.ts";

export interface OtherChoiceProps {
  readonly field: OwnField;
  /** Sends the ID chosen. */
  readonly onCommit: (id: string) => void;
}

/** *Group*: the groups the person may list, found by name. */
export function GroupChoice({ field, onCommit }: OtherChoiceProps) {
  const { frame } = usePickerFrame();
  const found = useGroups();
  const groups = groupLinesOf(found.data?.pages);
  return (
    <ChoiceDialog field={field}>
      {(done) => (
        <SearchPicker
          legend={t("fields.choose-title", { name: field.name })}
          searchLabel={t("fields.pick-search")}
          listLabel={t("fields.pick-groups-list")}
          {...frame}
          {...pickerStatus(found, groups.length, {
            nothing: t("fields.pick-no-groups"),
            more: t("fields.pick-more"),
          })}
        >
          {groups
            .filter((group) => group.name.toLowerCase().includes(frame.query.trim().toLowerCase()))
            .map((group) => (
              <PickerRow key={group.id} label={group.name}>
                <Button
                  aria-label={t("fields.pick-choose-name", { name: group.name })}
                  onPress={() => {
                    onCommit(group.id);
                    done();
                  }}
                >
                  {t("fields.choose")}
                </Button>
              </PickerRow>
            ))}
        </SearchPicker>
      )}
    </ChoiceDialog>
  );
}

/** *Issue*: the readable ID of an issue (like WEB-12), looked up before it is chosen. */
export function IssueChoice({ field, onCommit }: OtherChoiceProps) {
  const [key, setKey] = useState("");
  const [problem, setProblem] = useState<string | undefined>();
  const find = async (done: () => void) => {
    try {
      const found = dataOf(
        await api.client.GET("/api/v1/issues/{issue}", {
          params: { path: { issue: key.trim() } },
        }),
      );
      const id = found.issue.id;
      if (id === undefined || id === null) {
        setProblem(t("fields.pick-no-issue"));
        return;
      }
      setProblem(undefined);
      onCommit(id);
      done();
    } catch (error) {
      setProblem(messageOf(error));
    }
  };
  return (
    <ChoiceDialog field={field}>
      {(done) => (
        <Form
          onSubmit={() => {
            void find(done);
          }}
        >
          <TextField
            label={t("fields.pick-issue-key")}
            requiredMessage={t("validation.required")}
            value={key}
            onChange={setKey}
          />
          {problem === undefined ? null : <Alert tone={alertTone.danger}>{problem}</Alert>}
          <SubmitButton isPending={false}>{t("fields.pick-issue-find")}</SubmitButton>
        </Form>
      )}
    </ChoiceDialog>
  );
}
