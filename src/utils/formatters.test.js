import { describe, expect, it } from "vitest";

import { formatNextRaceCountdown } from "./formatters";

describe("formatNextRaceCountdown", () => {
  it("formats the countdown across months weeks and days", () => {
    expect(formatNextRaceCountdown(null)).toBe("Sem corrida pendente");
    expect(formatNextRaceCountdown(0)).toBe("Proxima corrida hoje");
    expect(formatNextRaceCountdown(1)).toBe("Proxima corrida amanha");
    expect(formatNextRaceCountdown(6)).toBe("Proxima corrida em 6 dias");
    expect(formatNextRaceCountdown(14)).toBe("Proxima corrida em 2 semanas");
    expect(formatNextRaceCountdown(28)).toBe("Proxima corrida em 1 mes");
    expect(formatNextRaceCountdown(56)).toBe("Proxima corrida em 2 meses");
  });
});
