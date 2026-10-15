import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { TestBed } from "@angular/core/testing";
import { ThemeService } from "./theme.service";

describe("ThemeService", () => {
  const originalMatchMedia = window.matchMedia;
  let systemChange: ((event: MediaQueryListEvent) => void) | undefined;

  beforeEach(() => {
    localStorage.removeItem("eattura.theme");
    systemChange = undefined;
    Object.defineProperty(window, "matchMedia", {
      configurable: true,
      value: vi.fn().mockReturnValue({
        matches: true,
        addEventListener: (_type: string, listener: (event: MediaQueryListEvent) => void) => { systemChange = listener; },
        removeEventListener: vi.fn(),
      }),
    });
  });

  afterEach(() => {
    Object.defineProperty(window, "matchMedia", { configurable: true, value: originalMatchMedia });
    localStorage.removeItem("eattura.theme");
    document.documentElement.removeAttribute("data-theme");
  });

  it("follows system changes while the preference is system", () => {
    const theme = TestBed.inject(ThemeService);
    expect(theme.effectiveTheme()).toBe("dark");
    expect(document.documentElement.dataset["theme"]).toBe("dark");

    systemChange?.({ matches: false } as MediaQueryListEvent);
    expect(theme.effectiveTheme()).toBe("light");
    expect(document.documentElement.dataset["theme"]).toBe("light");
  });

  it("persists explicit choices and restores system behavior", () => {
    const theme = TestBed.inject(ThemeService);
    theme.setPreference("light");
    expect(localStorage.getItem("eattura.theme")).toBe("light");

    systemChange?.({ matches: true } as MediaQueryListEvent);
    expect(theme.effectiveTheme()).toBe("light");

    theme.setPreference("system");
    expect(localStorage.getItem("eattura.theme")).toBeNull();
    expect(theme.effectiveTheme()).toBe("dark");
  });
});
