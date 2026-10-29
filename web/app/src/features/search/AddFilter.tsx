import { t } from "@bugaboo/messages";
import { Button, Icon, iconName } from "@bugaboo/ui";
import type { Field, ReadableFields } from "@bugaboo/wasm-search";
import { useState } from "react";

import { FieldChooser } from "./FieldChooser.tsx";

export interface AddFilterProps {
  readonly fields: ReadableFields;
  /** The person picked a field: its filter window opens. */
  readonly onPick: (field: Field) => void;
}

/**
 * *+ Add filter* (UX design section 4.3): the fields the person may read, the ones used most
 * first (the order the server gives), with a box to find the rest. A window on a computer, a
 * sheet from the bottom on a phone.
 */
export function AddFilter({ fields, onPick }: AddFilterProps) {
  const [open, setOpen] = useState(false);
  return (
    <>
      <Button
        onPress={() => {
          setOpen(true);
        }}
      >
        <Icon name={iconName.plus} />
        {t("search.bar.add-filter")}
      </Button>
      <FieldChooser
        isOpen={open}
        onOpenChange={setOpen}
        fields={fields}
        title={t("search.bar.add-filter")}
        onPick={onPick}
      />
    </>
  );
}
