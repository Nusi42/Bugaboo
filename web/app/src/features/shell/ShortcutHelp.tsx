import { t } from "@bugaboo/messages";
import { cx, IconButton, iconName, Overlay } from "@bugaboo/ui";
import { Link } from "react-aria-components";

import { paths } from "../../shared/paths.ts";
import { useShortcutOverrides } from "../../shared/use-shortcuts.ts";
import { closeOverlay, useOverlay } from "./overlay-state.ts";
import { ShortcutList } from "./ShortcutList.tsx";
import styles from "./ShortcutHelp.module.css";

/**
 * The list of all shortcuts, opened with `?` or the *Keyboard shortcuts* button (UX-05). Every
 * shortcut is optional: each one also has a visible button (principle 9).
 */
export function ShortcutHelp() {
  const overlay = useOverlay("shortcuts");
  const overrides = useShortcutOverrides();
  return (
    <Overlay {...overlay} label={t("shortcut.title")}>
      <div className={styles.help}>
        <div className={styles.head}>
          <h2 className={styles.title}>{t("shortcut.title")}</h2>
          <IconButton icon={iconName.close} label={t("common.close")} onPress={closeOverlay} />
        </div>
        <p className={styles.intro}>{t("shortcut.intro")}</p>
        <ShortcutList overrides={overrides} />
        <Link className={cx(styles.change)} href={paths.shortcuts} onPress={closeOverlay}>
          {t("shortcut.change-link")}
        </Link>
      </div>
    </Overlay>
  );
}
