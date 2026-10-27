import { t } from "@bugaboo/messages";
import { AskDialog, Button, Icon, iconName } from "@bugaboo/ui";
import type { Field, ReadableFields } from "@bugaboo/wasm-search";
import { useState } from "react";

import { FieldPicker } from "./FieldPicker.tsx";

export interface AddFilterProps {
  readonly fields: ReadableFields;
  /** The person picked a field: its filter window opens. */
  readonly onPick: (field: Field) => void;
}

/**
 * *+ Add filter* (UX design section 4.3): the fields the person may read, the ones used most
 * first (the order the server gives), with a box to find the rest.
 */
export function AddFilter({ fields, onPick }: AddFilterProps) {
  const [open, setOpen] = useState(false);
  const close = (): void => {
    setOpen(false);
  };
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
      <AskDialog
        isOpen={open}
        onOpenChange={setOpen}
        title={t("search.bar.add-filter")}
        text={t("search.bar.add-filter-which")}
        confirmLabel={t("common.close")}
        cancelLabel={t("common.cancel")}
        onConfirm={close}
      >
        <FieldPicker
          fields={fields}
          onPick={(field) => {
            close();
            onPick(field);
          }}
        />
      </AskDialog>
    </>
  );
}
