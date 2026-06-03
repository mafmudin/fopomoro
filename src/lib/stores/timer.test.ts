import { describe, it, expect } from "vitest";
import { formatClockTime, formatClockDate } from "./timer";

describe("clock formatting", () => {
  it("formats time as HH:mm:ss with zero padding", () => {
    const d = new Date(2026, 5, 3, 9, 7, 4); // 2026-06-03 09:07:04
    expect(formatClockTime(d)).toBe("09:07:04");
  });

  it("formats date as 'Weekday, DD Month YYYY'", () => {
    const d = new Date(2026, 5, 3, 9, 7, 4); // Wednesday, 03 June 2026
    expect(formatClockDate(d)).toBe("Wednesday, 03 June 2026");
  });

  it("zero-pads midnight to 00:00:00", () => {
    const d = new Date(2026, 5, 3, 0, 0, 0);
    expect(formatClockTime(d)).toBe("00:00:00");
  });
});
