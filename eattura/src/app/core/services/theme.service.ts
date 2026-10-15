import { DOCUMENT, isPlatformBrowser } from "@angular/common";
import { computed, DestroyRef, inject, Injectable, PLATFORM_ID, signal } from "@angular/core";

export type ThemePreference = "system" | "light" | "dark";

const STORAGE_KEY = "eattura.theme";

@Injectable({ providedIn: "root" })
export class ThemeService {
  private readonly document = inject(DOCUMENT);
  private readonly platformId = inject(PLATFORM_ID);
  private readonly destroyRef = inject(DestroyRef);
  private readonly preferenceState = signal<ThemePreference>("system");
  private readonly systemDark = signal(false);

  readonly preference = this.preferenceState.asReadonly();
  readonly effectiveTheme = computed(() =>
    this.preferenceState() === "system"
      ? (this.systemDark() ? "dark" : "light")
      : this.preferenceState(),
  );

  constructor() {
    if (!isPlatformBrowser(this.platformId)) return;

    const view = this.document.defaultView;
    const media = typeof view?.matchMedia === "function"
      ? view.matchMedia("(prefers-color-scheme: dark)")
      : undefined;
    this.systemDark.set(media?.matches ?? false);
    const onSystemChange = (event: MediaQueryListEvent): void => {
      this.systemDark.set(event.matches);
      this.applyTheme();
    };
    media?.addEventListener("change", onSystemChange);
    this.destroyRef.onDestroy(() => media?.removeEventListener("change", onSystemChange));

    try {
      const stored = this.document.defaultView?.localStorage.getItem(STORAGE_KEY);
      if (stored === "light" || stored === "dark") this.preferenceState.set(stored);
    } catch {
      // Storage may be unavailable in a restricted webview.
    }
    this.applyTheme();
  }

  setPreference(preference: ThemePreference): void {
    this.preferenceState.set(preference);
    if (isPlatformBrowser(this.platformId)) {
      try {
        if (preference === "system") this.document.defaultView?.localStorage.removeItem(STORAGE_KEY);
        else this.document.defaultView?.localStorage.setItem(STORAGE_KEY, preference);
      } catch {
        // The active preference still applies for this session.
      }
      this.applyTheme();
    }
  }

  private applyTheme(): void {
    this.document.documentElement.dataset["theme"] = this.effectiveTheme();
    this.document.documentElement.style.colorScheme = this.effectiveTheme();
  }
}
