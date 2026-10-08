import { t } from "@bugaboo/messages";
import { Button, buttonLook, Card, partOf, part } from "@bugaboo/ui";

import { usePageTitle } from "../../shared/use-page.ts";
import { FocusView } from "./FocusView.tsx";
import { ListEditor, type ListRow } from "./ListEditor.tsx";
import { useExamples, type Example } from "./use-examples.ts";

/** How an item is shown: in the side panel next to the list, or in the focus view. */
export type FramesShow = "panel" | "focus";

export interface FramesPageProps {
  /** How the open item is shown, from the address. */
  readonly show: FramesShow | undefined;
  /** The open item, from the address. */
  readonly item: string | undefined;
  readonly onOpen: (show: FramesShow, item: string) => void;
  readonly onClose: () => void;
}

interface SlimListProps {
  readonly items: readonly Example[];
  readonly open: string;
  readonly onOpen: (id: string) => void;
}

/** The slim list of the focus view: the items around the open one. */
function SlimList({ items, open, onOpen }: SlimListProps) {
  return (
    <ul {...partOf(part.framesPageSlim)} aria-label={t("frames.list")}>
      {items.map((example) => (
        <li key={example.id}>
          <Button
            look={example.id === open ? buttonLook.plain : buttonLook.quiet}
            current={example.id === open}
            onPress={() => {
              onOpen(example.id);
            }}
          >
            {example.name}
          </Button>
        </li>
      ))}
    </ul>
  );
}

/** What can be done with an example next to its name: open it in the focus view, or remove it. */
function ExampleActions({
  example,
  onFocus,
  onRemove,
}: {
  readonly example: Example;
  readonly onFocus: () => void;
  readonly onRemove: () => void;
}) {
  return (
    <span {...partOf(part.framesPageActions)}>
      <Button aria-label={t("frames.open-focus", { name: example.name })} onPress={onFocus}>
        {t("frames.focus-view")}
      </Button>
      <Button aria-label={t("frames.remove-name", { name: example.name })} onPress={onRemove}>
        {t("frames.remove")}
      </Button>
    </span>
  );
}

/** The item that is open in the focus view, next to the slim list. */
function Focused({
  items,
  open,
  onOpen,
  onClose,
}: SlimListProps & Pick<FramesPageProps, "onClose">) {
  const current = items.find((example) => example.id === open);
  const step = (direction: 1 | -1): void => {
    const at = items.findIndex((example) => example.id === open);
    const next = items[(at + direction + items.length) % items.length];
    if (next !== undefined) {
      onOpen(next.id);
    }
  };
  return (
    <FocusView
      list={<SlimList items={items} open={open} onOpen={onOpen} />}
      onBack={onClose}
      onStep={step}
    >
      <h2 {...partOf(part.framesPageItemTitle)}>{current?.name}</h2>
      <p {...partOf(part.text)}>{t("frames.focus-text")}</p>
    </FocusView>
  );
}

/**
 * The frames that later screens open in (UX §3.4): the side panel next to a list, and the focus
 * view that gives the open item the whole area. They are empty here; the page exists so the frames
 * can be tried, and checked, before issues arrive. *Remove* shows the 10 seconds of *Undo*.
 */
export function FramesPage({ show, item, onOpen, onClose }: FramesPageProps) {
  usePageTitle(t("frames.title"));
  const { items, remove } = useExamples();
  const rows: readonly ListRow[] = items.map((example) => ({
    key: example.id,
    name: example.name,
    details: (
      <ExampleActions
        example={example}
        onFocus={() => {
          onOpen("focus", example.id);
        }}
        onRemove={() => {
          remove(example);
        }}
      />
    ),
    editor: <p {...partOf(part.text)}>{t("frames.panel-text")}</p>,
  }));
  const focused = show === "focus" && items.some((example) => example.id === item);
  return (
    <Card heading={t("frames.title")} level={1}>
      <p {...partOf(part.text)}>{t("frames.intro")}</p>
      {focused && item !== undefined ? (
        <Focused
          items={items}
          open={item}
          onOpen={(id) => {
            onOpen("focus", id);
          }}
          onClose={onClose}
        />
      ) : (
        <ListEditor
          rows={rows}
          searchLabel={t("frames.list")}
          nothing={t("frames.nothing")}
          openKey={show === "panel" ? item : undefined}
          onOpen={(key) => {
            onOpen("panel", key);
          }}
          onClose={onClose}
        />
      )}
    </Card>
  );
}
