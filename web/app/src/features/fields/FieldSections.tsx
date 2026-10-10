import { BasicsSection } from "./BasicsSection.tsx";
import { ChoicesSection } from "./ChoicesSection.tsx";
import { useFieldTypes, type Field } from "./fields.ts";
import { LookSection } from "./LookSection.tsx";
import { RulesSection } from "./RulesSection.tsx";
import { UsageSection } from "./UsageSection.tsx";

/** The sections of the page of a field that edit it: basics, choices, rules, look and where it is used. */
export function FieldSections({ field }: { readonly field: Field }) {
  const types = useFieldTypes();
  const hasChoices = types.data?.kinds
    .find((each) => each.kind === field.kind)
    ?.traits.includes("has-choices");
  return (
    <>
      <BasicsSection field={field} />
      {hasChoices === true ? <ChoicesSection field={field} /> : null}
      <RulesSection field={field} />
      <LookSection field={field} />
      <UsageSection field={field} />
    </>
  );
}
