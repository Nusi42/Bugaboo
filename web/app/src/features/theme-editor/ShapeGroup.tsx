import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";

import { CornersAndFaces } from "./CornersAndFaces.tsx";
import { ShadowLayers, type ShadowLayersProps } from "./ShadowLayers.tsx";

/**
 * *Shadow, corners and fonts* (UX §5.6, M8-Q10): the corners of controls and of cards, the faces of
 * text, headings and long text among the built-in ones, and the shadow as layers. Nothing else: no
 * spacing and no font files. It starts closed.
 */
export function ShapeGroup(props: ShadowLayersProps) {
  return (
    <details {...partOf(part.themeGroup)}>
      <summary {...partOf(part.themeGroupSummary)}>{t("theme-editor.group.shape")}</summary>
      <div {...partOf(part.themeGroupBody)}>
        <CornersAndFaces values={props.values} accepts={props.accepts} onChange={props.onChange} />
        <ShadowLayers {...props} />
      </div>
    </details>
  );
}
