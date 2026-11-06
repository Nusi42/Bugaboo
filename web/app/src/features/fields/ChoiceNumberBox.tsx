import { choiceNumberDigits } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { TextField, partOf, part } from "@bugaboo/ui";

/** The box of the number of a choice, in the code font, as wide as a number needs: on the page of a field, for a choice and for a new one. */
export function ChoiceNumberBox({
  text,
  onChange,
  invalid,
  problem,
}: {
  readonly text: string;
  readonly onChange: (text: string) => void;
  readonly invalid: boolean;
  /** What the server said about the number, when it refused it (another choice has it). */
  readonly problem?: string | undefined;
}) {
  return (
    <span {...partOf(part.lengthBox, { variants: ["number"] })}>
      <TextField
        label={t("fields.choice-number")}
        description={t("fields.choice-number-hint", { max: choiceNumberDigits })}
        errorMessage={
          invalid ? t("fields.choice-number-invalid", { max: choiceNumberDigits }) : problem
        }
        value={text}
        onChange={onChange}
      />
    </span>
  );
}
