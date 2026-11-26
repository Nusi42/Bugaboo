import { boxHeightLeast, gridColumns, type Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Button, buttonLook, part, partOf, Window } from "@bugaboo/ui";

import { windowLabels } from "../../shared/window-labels.ts";
import { BoxContent } from "./BoxContent.tsx";
import { BoxForm } from "./BoxForm.tsx";
import {
  bodyOf,
  boxKind,
  kindName,
  useBoxPreview,
  useReadyMadeBoxes,
  type AskedBox,
} from "./dashboards.ts";
import { placeAtEnd, type GridPlace } from "./grid-layout.ts";

/** A box asked for where a new box goes: under every box, in its own size. */
function atEnd(box: AskedBox, places: readonly GridPlace[]): AskedBox {
  return { ...box, id: null, ...placeAtEnd(places, box) };
}

/** A box of your own as its form starts: the first issues of a search, half the grid wide. */
const ownStart: AskedBox = {
  kind: boxKind.searchResults,
  title: "",
  search_text: "",
  column: 0,
  row: 0,
  width: gridColumns / 2,
  height: boxHeightLeast * 2,
};

/** One ready-made box: its name, its kind, a live preview of it for the person, and *Add*. */
function ReadyMadeChoice({
  item,
  places,
  isOpen,
  onAdd,
}: {
  readonly item: Schema<"ReadyMadeItem">;
  readonly places: readonly GridPlace[];
  readonly isOpen: boolean;
  readonly onAdd: (box: AskedBox) => void;
}) {
  const asked = atEnd(bodyOf(item.box), places);
  const preview = useBoxPreview(asked, isOpen);
  return (
    <li {...partOf(part.addBoxChoice)}>
      <h3 {...partOf(part.formSectionTitle)}>{item.box.title}</h3>
      <p {...partOf(part.addBoxCaption)}>{t(kindName[item.box.kind])}</p>
      <div {...partOf(part.addBoxPreview)} inert>
        <BoxContent box={item.box} read={preview} />
      </div>
      <Button
        look={buttonLook.quiet}
        onPress={() => {
          onAdd(asked);
        }}
      >
        {t("dashboards.add-this", { title: item.box.title })}
      </Button>
    </li>
  );
}

export interface AddBoxWindowProps {
  readonly isOpen: boolean;
  readonly onClose: () => void;
  /** The places of the boxes the person sees: a new box goes under them. */
  readonly places: readonly GridPlace[];
  readonly pending: boolean;
  readonly onAdd: (box: AskedBox) => void;
}

/**
 * *Add box* (UX §4.10, ADR-0050, decision 4): the ready-made boxes, each with a live preview made by the same read as
 * the box, then a box of your own: one of the six kinds with a search and the options of its kind.
 */
export function AddBoxWindow({ isOpen, onClose, places, pending, onAdd }: AddBoxWindowProps) {
  const ready = useReadyMadeBoxes();
  return (
    <Window
      isOpen={isOpen}
      onOpenChange={(open) => {
        if (!open) {
          onClose();
        }
      }}
      title={t("dashboards.add-box")}
      labels={windowLabels()}
    >
      <div {...partOf(part.addBox)}>
        <section>
          <h3 {...partOf(part.formSectionTitle)}>{t("dashboards.ready-made")}</h3>
          <ul {...partOf(part.addBoxList)}>
            {(ready.data?.boxes ?? []).map((item) => (
              <ReadyMadeChoice
                key={item.which}
                item={item}
                places={places}
                isOpen={isOpen}
                onAdd={onAdd}
              />
            ))}
          </ul>
        </section>
        <section>
          <h3 {...partOf(part.formSectionTitle)}>{t("dashboards.own-box")}</h3>
          <BoxForm
            start={ownStart}
            chooseKind
            submit={t("dashboards.add-the-box")}
            pending={pending}
            onSubmit={(box) => {
              onAdd(atEnd(box, places));
            }}
          />
        </section>
      </div>
    </Window>
  );
}
