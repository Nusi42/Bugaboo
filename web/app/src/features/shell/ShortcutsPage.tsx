import { t } from "@bugaboo/messages";
import { Card } from "@bugaboo/ui";

import { keysOf, shortcutDefinitions, type ShortcutId } from "../../shared/shortcuts.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { useShortcutOverrides } from "../../shared/use-shortcuts.ts";
import { Keys } from "./Keys.tsx";
import { ListEditor, type ListRow } from "./ListEditor.tsx";
import { ShortcutEditor } from "./ShortcutEditor.tsx";
import styles from "./ShortcutsPage.module.css";

export interface ShortcutsPageProps {
  /** The shortcut that is open in the side panel, from the address. */
  readonly edit: ShortcutId | undefined;
  readonly onEdit: (id: ShortcutId) => void;
  readonly onClose: () => void;
}

/**
 * *My settings › Keyboard shortcuts* (UX-05): a searchable list of every shortcut; one that is
 * opened shows in the side panel, where its keys can be changed. It is the list → editor pattern
 * of UX §5.1 on the simplest items there are.
 */
export function ShortcutsPage({ edit, onEdit, onClose }: ShortcutsPageProps) {
  usePageTitle(t("shortcuts.title"));
  const overrides = useShortcutOverrides();
  const rows: readonly ListRow[] = shortcutDefinitions.map((definition) => ({
    key: definition.id,
    name: t(definition.message),
    details: (
      <span className={styles.keys}>
        {keysOf(definition.id, overrides).map((sequence) => (
          <Keys key={sequence} sequence={sequence} />
        ))}
      </span>
    ),
    editor: <ShortcutEditor id={definition.id} overrides={overrides} />,
  }));
  return (
    <Card heading={t("shortcuts.title")} level={1}>
      <p className={styles.intro}>{t("shortcuts.intro")}</p>
      <ListEditor
        rows={rows}
        searchLabel={t("shortcuts.search")}
        nothing={t("shortcuts.nothing")}
        openKey={edit}
        onOpen={(key) => {
          const found = shortcutDefinitions.find((definition) => definition.id === key);
          if (found !== undefined) {
            onEdit(found.id);
          }
        }}
        onClose={onClose}
      />
    </Card>
  );
}
