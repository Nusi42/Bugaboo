import { t } from "@bugaboo/messages";

import { minutesText } from "../../shared/own-value-text.ts";
import { renderSaid } from "../../shared/said.ts";
import { WithLengthReader, type LengthReader } from "../../shared/with-length-reader.tsx";
import { type SettingControlProps } from "./SettingControl.tsx";
import { SettingBox } from "./SettingBox.tsx";
import { useSettingBox } from "./use-setting-box.ts";

/** What a box for a length needs besides the setting: the one reader of lengths. */
interface LengthBoxProps extends SettingControlProps {
  readonly readLength: LengthReader;
}

function Box({ setting, onChange, help, source, readLength }: LengthBoxProps) {
  const bounds = setting.bounds ?? undefined;
  const between =
    bounds === undefined
      ? undefined
      : t("setting.ui.between-amounts", {
          least: minutesText(bounds.least),
          most: minutesText(bounds.most),
        });
  // The standard value in words, where this level changed it: the number to go back to.
  const extra =
    setting.own && typeof setting.default === "number" && setting.default !== setting.value
      ? t("setting.ui.standard-length", { length: minutesText(setting.default) })
      : undefined;
  const box = useSettingBox({
    setting,
    help,
    between,
    extra,
    source,
    initial: minutesText(typeof setting.value === "number" ? setting.value : 0),
    read: (text) => {
      const { minutes, mistake } = readLength(text);
      if (minutes !== null) {
        return { value: minutes };
      }
      return { problem: mistake === null ? t("common.words-failed") : renderSaid(mistake) };
    },
    onChange,
  });
  return <SettingBox {...box} isLength />;
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
