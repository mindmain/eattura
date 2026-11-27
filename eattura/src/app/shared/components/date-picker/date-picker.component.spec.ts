import { Component } from "@angular/core";
import { TestBed } from "@angular/core/testing";
import { FormControl, ReactiveFormsModule } from "@angular/forms";
import { readFileSync } from "node:fs";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { DatePickerComponent } from "./date-picker.component";

@Component({
  standalone: true,
  imports: [ReactiveFormsModule, DatePickerComponent],
  template: `
    <app-date-picker
      [minDate]="'2026-11-05'"
      [maxDate]="'2026-11-28'"
      [formControl]="control"
    />
  `,
})
class TestHostDateFormComponent {
  readonly control = new FormControl<string>("2026-11-27");
}

describe("DatePickerComponent", () => {
  beforeEach(async () => {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({});
    TestBed.overrideComponent(DatePickerComponent, {
      set: {
        templateUrl: undefined,
        styleUrl: undefined,
        template: readFileSync(
          "src/app/shared/components/date-picker/date-picker.component.html",
          "utf8"
        ),
        styles: [],
      },
    });
    await TestBed.compileComponents();
  });

  it("formats ISO YYYY-MM-DD dates in Italian dd/mm/yyyy format and shows placeholder when empty", () => {
    const fixture = TestBed.createComponent(DatePickerComponent);
    fixture.detectChanges();

    const trigger: HTMLButtonElement = fixture.nativeElement.querySelector(".date-picker-trigger");
    expect(trigger.textContent?.trim()).toBe("gg/mm/aaaa");

    fixture.componentRef.setInput("value", "2026-11-27");
    fixture.detectChanges();

    expect(fixture.componentInstance.formattedValue()).toBe("27/11/2026");
    expect(trigger.querySelector(".date-value")?.textContent?.trim()).toBe("27/11/2026");
  });

  it("renders Monday-first Italian calendar grid and navigates months and years", () => {
    const fixture = TestBed.createComponent(DatePickerComponent);
    fixture.componentRef.setInput("value", "2026-11-27");
    fixture.detectChanges();

    const trigger: HTMLButtonElement = fixture.nativeElement.querySelector(".date-picker-trigger");
    trigger.click();
    fixture.detectChanges();

    const hostEl = fixture.nativeElement as HTMLElement;
    const weekdays = Array.from(hostEl.querySelectorAll<HTMLElement>(".weekday-cell")).map((el) =>
      el.textContent?.trim()
    );
    expect(weekdays).toEqual(["Lun", "Mar", "Mer", "Gio", "Ven", "Sab", "Dom"]);

    // November 1, 2026 is a Sunday -> Monday-first grid starts on Monday Oct 26, 2026
    const firstCell = hostEl.querySelector<HTMLButtonElement>(".day-cell")!;
    expect(firstCell.getAttribute("data-iso")).toBe("2026-10-26");

    const monthSelect = hostEl.querySelector<HTMLSelectElement>(".month-select")!;
    monthSelect.value = "1";
    monthSelect.dispatchEvent(new Event("change"));
    fixture.detectChanges();
    expect(fixture.componentInstance.viewMonth()).toBe(1);

    const yearSelect = hostEl.querySelector<HTMLSelectElement>(".year-select")!;
    yearSelect.value = "2028";
    yearSelect.dispatchEvent(new Event("change"));
    fixture.detectChanges();
    expect(fixture.componentInstance.viewYear()).toBe(2028);

    const prevBtn = hostEl.querySelector<HTMLButtonElement>('[aria-label="Mese precedente"]')!;
    prevBtn.click();
    fixture.detectChanges();
    expect(fixture.componentInstance.viewMonth()).toBe(0);
    expect(fixture.componentInstance.viewYear()).toBe(2028);

    prevBtn.click();
    fixture.detectChanges();
    expect(fixture.componentInstance.viewMonth()).toBe(11);
    expect(fixture.componentInstance.viewYear()).toBe(2027);

    const nextBtn = hostEl.querySelector<HTMLButtonElement>('[aria-label="Mese successivo"]')!;
    nextBtn.click();
    fixture.detectChanges();
    expect(fixture.componentInstance.viewMonth()).toBe(0);
    expect(fixture.componentInstance.viewYear()).toBe(2028);
  });

  it("enforces minDate and maxDate boundaries and emits selected ISO date", () => {
    const fixture = TestBed.createComponent(DatePickerComponent);
    fixture.componentRef.setInput("value", "2026-11-15");
    fixture.componentRef.setInput("minDate", "2026-11-10");
    fixture.componentRef.setInput("maxDate", "2026-11-20");
    const emitted = vi.fn();
    fixture.componentInstance.valueChange.subscribe(emitted);
    fixture.detectChanges();

    fixture.componentInstance.openPopover();
    fixture.detectChanges();

    const tooEarly: HTMLButtonElement = fixture.nativeElement.querySelector(
      '[data-iso="2026-11-09"]'
    );
    const allowed: HTMLButtonElement = fixture.nativeElement.querySelector(
      '[data-iso="2026-11-18"]'
    );
    const tooLate: HTMLButtonElement = fixture.nativeElement.querySelector(
      '[data-iso="2026-11-21"]'
    );

    expect(tooEarly.disabled).toBe(true);
    expect(tooLate.disabled).toBe(true);
    expect(allowed.disabled).toBe(false);

    allowed.click();
    fixture.detectChanges();
    expect(emitted).toHaveBeenCalledWith("2026-11-18");
    expect(fixture.componentInstance.isOpen()).toBe(false);
    expect(fixture.componentInstance.formattedValue()).toBe("18/11/2026");
  });

  it("supports Today button, Escape key, backdrop close, and Reactive Forms CVA integration", () => {
    const fixture = TestBed.createComponent(TestHostDateFormComponent);
    fixture.detectChanges();

    const host = fixture.componentInstance;
    const el: HTMLElement = fixture.nativeElement;
    expect(el.querySelector(".date-value")?.textContent?.trim()).toBe("27/11/2026");

    const trigger: HTMLButtonElement = el.querySelector(".date-picker-trigger")!;
    trigger.click();
    fixture.detectChanges();

    el.querySelector("app-date-picker")!.dispatchEvent(
      new KeyboardEvent("keydown", { key: "Escape", bubbles: true })
    );
    fixture.detectChanges();
    expect(el.querySelector(".date-picker-popover")).toBeNull();

    const clearBtn: HTMLButtonElement = el.querySelector(".date-clear-btn")!;
    clearBtn.click();
    fixture.detectChanges();
    expect(host.control.value).toBe("");

    host.control.disable();
    fixture.detectChanges();
    expect(trigger.disabled).toBe(true);
    expect(el.querySelector(".date-clear-btn")).toBeNull();
  });
});
