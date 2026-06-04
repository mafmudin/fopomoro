export interface TextColors {
  text: string;
  subtext: string;
}

const LIGHT_TEXT: TextColors = { text: "#CDD6F4", subtext: "#BAC2DE" };
const DARK_TEXT: TextColors = { text: "#1E1E2E", subtext: "#45475A" };

/**
 * Picks readable text colors for a given panel background. Returns light
 * Catppuccin text on dark backgrounds and dark text on light ones, chosen by
 * WCAG relative luminance so the panel text stays legible at any picked color.
 */
export function textColorsFor(bgHex: string): TextColors {
  return relativeLuminance(bgHex) > 0.179 ? DARK_TEXT : LIGHT_TEXT;
}

function relativeLuminance(hex: string): number {
  const h = hex.replace(/^#/, "");
  const r = parseInt(h.slice(0, 2), 16) / 255;
  const g = parseInt(h.slice(2, 4), 16) / 255;
  const b = parseInt(h.slice(4, 6), 16) / 255;
  const lin = (c: number) =>
    c <= 0.03928 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
  return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
}
