import { describe, it, expect } from "vitest";
import { textColorsFor } from "./contrast";

const LIGHT = { text: "#CDD6F4", subtext: "#BAC2DE" };
const DARK = { text: "#1E1E2E", subtext: "#45475A" };

describe("textColorsFor", () => {
  it("uses light text on a dark background", () => {
    expect(textColorsFor("#1E1E2E")).toEqual(LIGHT);
  });

  it("uses dark text on a light background", () => {
    expect(textColorsFor("#FFFFFF")).toEqual(DARK);
  });

  it("uses light text on every dark preset", () => {
    for (const bg of ["#181825", "#11111B", "#24273A", "#303446"]) {
      expect(textColorsFor(bg)).toEqual(LIGHT);
    }
  });

  it("uses dark text on a light gray", () => {
    expect(textColorsFor("#E8E8E8")).toEqual(DARK);
  });

  it("tolerates a hex without the leading #", () => {
    expect(textColorsFor("1E1E2E")).toEqual(LIGHT);
  });
});
