import { t } from "@bugaboo/messages";
import { LongPreview, type LongPreviewProps } from "@bugaboo/ui";

/** The long preview as the app uses it: the words of *Show all* and *Show fewer* are set once. */
export function AppLongPreview(props: Omit<LongPreviewProps, "showAll" | "showFewer">) {
  return (
    <LongPreview
      {...props}
      showAll={(count) => t("common.show-all", { count })}
      showFewer={t("common.show-fewer")}
    />
  );
}
