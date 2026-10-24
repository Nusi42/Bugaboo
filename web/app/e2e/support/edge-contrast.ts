import { type Locator } from "@playwright/test";

/** The three numbers of a color the browser writes as `rgb(1, 2, 3)` or `rgba(1, 2, 3, 0.5)`. */
function channels(color: string): readonly number[] {
  return (color.match(/[\d.]+/gu) ?? []).slice(0, 3).map(Number);
}

const SRGB_STEP = 0.04045;

/** How bright a color is, by the formula of WCAG. */
function luminance(parts: readonly number[]): number {
  const [red = 0, green = 0, blue = 0] = parts.map((value) => {
    const unit = value / 255;
    return unit <= SRGB_STEP ? unit / 12.92 : ((unit + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * red + 0.7152 * green + 0.0722 * blue;
}

/**
 * The contrast, by the formula of WCAG, between the page and the edge of a button as it is seen: the
 * color of the edge, faded by the opacity of the button, on the color of the page.
 */
export async function edgeContrast(button: Locator): Promise<number> {
  const { edge, page, opacity } = await button.evaluate((element) => {
    const style = getComputedStyle(element);
    return {
      edge: style.borderTopColor,
      page: getComputedStyle(document.body).backgroundColor,
      opacity: Number(style.opacity),
    };
  });
  const behind = channels(page);
  const seen = channels(edge).map(
    (value, at) => value * opacity + (behind[at] ?? 0) * (1 - opacity),
  );
  const [high = 0, low = 0] = [luminance(seen), luminance(behind)].toSorted((a, b) => b - a);
  return (high + 0.05) / (low + 0.05);
}
