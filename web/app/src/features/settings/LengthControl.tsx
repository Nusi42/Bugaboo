import { t } from "@bugaboo/messages";
import { Form, SubmitButton, TextField, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { minutesText } from "../../shared/own-value-text.ts";
import { renderSaid } from "../../shared/said.ts";
import { WithLengthReader, type LengthReader } from "../../shared/with-length-reader.tsx";
import { type SettingControlProps } from "./SettingControl.tsx";
import { wordsOf } from "./texts.ts";

/** What a box for a length needs besides the setting: the one reader of lengths. */
interface LengthBoxProps extends SettingControlProps {
  readonly readLength: LengthReader;
}

/** The sentence that says which lengths a setting accepts, in the words of a length. */
function betweenIn(setting: SettingControlProps["setting"]): string | undefined {
  const bounds = setting.bounds ?? undefined;
  return bounds === undefined
    ? undefined
    : t("setting.ui.between-lengths", {
        least: minutesText(bounds.least),
        most: minutesText(bounds.most),
      });
}

/** The standard value in words, where this level changed it: the number to go back to. */
function standardIn(setting: SettingControlProps["setting"]): string | undefined {
  return setting.own && typeof setting.default === "number" && setting.default !== setting.value
    ? t("setting.ui.standard-length", { length: minutesText(setting.default) })
    : undefined;
}

function Box({ setting, onChange, help, source, readLength }: LengthBoxProps) {
  const value = typeof setting.value === "number" ? setting.value : 0;
  const [draft, setDraft] = useState(minutesText(value));
  const [problem, setProblem] = useState<string>();
  const bounds = setting.bounds ?? undefined;
  const between = betweenIn(setting);
  const description =
    [help, between, standardIn(setting)].filter((piece) => piece !== undefined).join(" ") ||
    undefined;
  const submit = (): void => {
    const { minutes, mistake } = readLength(draft);
    if (minutes === null) {
      setProblem(mistake === null ? t("common.words-failed") : renderSaid(mistake));
      return;
    }
    // A length outside the bounds is not sent: the server would only refuse it.
    if (bounds !== undefined && (minutes < bounds.least || minutes > bounds.most)) {
      setProblem(between);
      return;
    }
    setProblem(undefined);
    onChange(minutes);
  };
  return (
    <Form onSubmit={submit}>
      <TextField
        label={wordsOf(setting.name_key)}
        {...(description === undefined ? {} : { description })}
        errorMessage={problem}
        value={draft}
        onChange={(next) => {
          setDraft(next);
          setProblem(undefined);
        }}
      />
      {source === undefined ? null : <p {...partOf(part.settingsSource)}>{source}</p>}
      <div {...partOf(part.row)}>
        <SubmitButton>{t("common.save")}</SubmitButton>
      </div>
    </Form>
  );
}

/**
 * A setting that counts minutes, drawn and typed as a length (`10h`, `1:30`, `90m`) with the one
 * reader of lengths of `bb-core`, like the lengths of issues (ADR-0035); the registry says which
 * numbers are minutes (ADR-0018). The server still gets whole minutes.
 */
export function LengthControl(props: SettingControlProps) {
  return (
    <WithLengthReader>
      {(readLength) => <Box {...props} readLength={readLength} />}
    </WithLengthReader>
  );
}
