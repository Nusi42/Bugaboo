import { t } from "@bugaboo/messages";
import { Button, buttonLook, Overlay, overlayEdge, part, partOf, useLayout } from "@bugaboo/ui";
import type { Field, ReadableFields } from "@bugaboo/wasm-search";

import { FieldPicker } from "./FieldPicker.tsx";

export interface FieldChooserProps {
  readonly isOpen: boolean;
  readonly onOpenChange: (open: boolean) => void;
  readonly fields: ReadableFields;
  /** The title of the window: *Add filter*, or *Add a filter* inside an *Any of*. */
  readonly title: string;
  /** The person picked a field; the window closes. */
  readonly onPick: (field: Field) => void;
}

/**
 * The window that lists the fields the person may read, the ones used most first (the order the
 * server gives), with a box to find the rest: a window on a computer, a sheet from the bottom on
 * a phone. *Add filter* and *Add a filter* of an *Any of* both open it.
 */
export function FieldChooser({ isOpen, onOpenChange, fields, title, onPick }: FieldChooserProps) {
  const phone = useLayout() === "phone";
  return (
    <Overlay
      isOpen={isOpen}
      onOpenChange={onOpenChange}
      label={title}
      from={phone ? overlayEdge.bottom : overlayEdge.center}
    >
      <div {...partOf(part.searchPanel)}>
        <h2 {...partOf(part.confirmTitle)}>{title}</h2>
        <p {...partOf(part.muted)}>{t("search.bar.add-filter-which")}</p>
        <FieldPicker
          fields={fields}
          onPick={(field) => {
            onOpenChange(false);
            onPick(field);
          }}
        />
        <div {...partOf(part.searchPanelActions)}>
          <Button
            look={buttonLook.quiet}
            onPress={() => {
              onOpenChange(false);
            }}
          >
            {t("common.cancel")}
          </Button>
        </div>
      </div>
    </Overlay>
  );
}
