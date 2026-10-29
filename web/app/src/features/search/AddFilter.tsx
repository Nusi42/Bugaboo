import { t } from "@bugaboo/messages";
import {
  Button,
  buttonLook,
  Icon,
  iconName,
  Overlay,
  overlayEdge,
  part,
  partOf,
  useLayout,
} from "@bugaboo/ui";
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
 * first (the order the server gives), with a box to find the rest. A window on a computer, a
 * sheet from the bottom on a phone.
 */
export function AddFilter({ fields, onPick }: AddFilterProps) {
  const [open, setOpen] = useState(false);
  const phone = useLayout() === "phone";
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
      <Overlay
        isOpen={open}
        onOpenChange={setOpen}
        label={t("search.bar.add-filter")}
        from={phone ? overlayEdge.bottom : overlayEdge.center}
      >
        <div {...partOf(part.searchPanel)}>
          <h2 {...partOf(part.confirmTitle)}>{t("search.bar.add-filter")}</h2>
          <p {...partOf(part.muted)}>{t("search.bar.add-filter-which")}</p>
          <FieldPicker
            fields={fields}
            onPick={(field) => {
              setOpen(false);
              onPick(field);
            }}
          />
          <div {...partOf(part.searchPanelActions)}>
            <Button
              look={buttonLook.quiet}
              onPress={() => {
                setOpen(false);
              }}
            >
              {t("common.cancel")}
            </Button>
          </div>
        </div>
      </Overlay>
    </>
  );
}
