import { Component } from "@angular/core";
import { TestBed } from "@angular/core/testing";
import { FormControl, ReactiveFormsModule } from "@angular/forms";
import { readFileSync } from "node:fs";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { DocSelectComponent, type DocSelectOption } from "./doc-select.component";

const NATURA_OPTIONS: readonly DocSelectOption<string>[] = [
  { value: "N1", code: "N1", label: "Escluse ex art. 15", detail: "Spese anticipate in nome e per conto del cliente" },
  { value: "N2.1", code: "N2.1", label: "Non soggette ad IVA", detail: "Articoli da 7 a 7-septies del DPR 633/72" },
  { value: "N2.2", code: "N2.2", label: "Non soggette – altri casi", detail: "Regime forfettario o minimi" },
  { value: "N3.1", code: "N3.1", label: "Non imponibili – esportazioni", detail: "Esportazioni dirette extra-UE" },
  { value: "N4", code: "N4", label: "Esenti", detail: "Operazioni esenti ex art. 10 DPR 633/72" },
  { value: "N6.1", code: "N6.1", label: "Inversione contabile – rottami", detail: "Cessione di rottami e materiali di recupero" },
  { value: "N6.9", code: "N6.9", label: "Inversione contabile – altri casi", detail: "Reverse charge interno" },
];

@Component({
  standalone: true,
  imports: [ReactiveFormsModule, DocSelectComponent],
  template: `
    <app-doc-select
      [options]="options"
      [clearable]="true"
      [emptyLabel]="'Nessuna natura'"
      [formControl]="control"
    />
  `,
})
class TestHostFormComponent {
  readonly options = NATURA_OPTIONS;
  readonly control = new FormControl<string>("N2.2");
}

describe("DocSelectComponent", () => {
  beforeEach(async () => {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({});
    TestBed.overrideComponent(DocSelectComponent, {
      set: {
        templateUrl: undefined,
        styleUrl: undefined,
        template: readFileSync(
          "src/app/shared/components/doc-select/doc-select.component.html",
          "utf8"
        ),
        styles: [],
      },
    });
    await TestBed.compileComponents();
  });

  it("renders placeholder when empty and code + label when an option is selected", () => {
    const fixture = TestBed.createComponent(DocSelectComponent);
    fixture.componentRef.setInput("options", NATURA_OPTIONS.slice(0, 3));
    fixture.componentRef.setInput("placeholder", "Seleziona natura…");
    fixture.detectChanges();

    const trigger: HTMLButtonElement = fixture.nativeElement.querySelector(".doc-select-trigger");
    expect(trigger.textContent).toContain("Seleziona natura…");

    fixture.componentRef.setInput("value", "N2.2");
    fixture.detectChanges();

    const badge = trigger.querySelector(".doc-code-badge");
    const label = trigger.querySelector(".doc-trigger-label");
    expect(badge?.textContent?.trim()).toBe("N2.2");
    expect(label?.textContent?.trim()).toBe("Non soggette – altri casi");
  });

  it("handles numeric zero values (e.g. 0% VAT rate) without treating 0 as empty", () => {
    const vatOptions: readonly DocSelectOption<number>[] = [
      { value: 22, code: "22%", label: "Aliquota ordinaria" },
      { value: 0, code: "0%", label: "Operazione senza addebito IVA" },
    ];
    const fixture = TestBed.createComponent(DocSelectComponent);
    fixture.componentRef.setInput("options", vatOptions);
    fixture.componentRef.setInput("value", 0);
    fixture.detectChanges();

    expect(fixture.componentInstance.hasSelectedValue()).toBe(true);
    const badge = fixture.nativeElement.querySelector(".doc-code-badge");
    expect(badge?.textContent?.trim()).toBe("0%");
  });

  it("enables search automatically when options exceed 6 and filters by code, label, and detail", () => {
    const fixture = TestBed.createComponent(DocSelectComponent);
    fixture.componentRef.setInput("options", NATURA_OPTIONS);
    fixture.detectChanges();

    const trigger: HTMLButtonElement = fixture.nativeElement.querySelector(".doc-select-trigger");
    trigger.click();
    fixture.detectChanges();

    const searchInput: HTMLInputElement | null =
      fixture.nativeElement.querySelector(".doc-search-input");
    expect(searchInput).not.toBeNull();

    searchInput!.value = "forfettario";
    searchInput!.dispatchEvent(new Event("input"));
    fixture.detectChanges();

    const rows = fixture.nativeElement.querySelectorAll(".doc-option-row");
    expect(rows.length).toBe(1);
    expect(rows[0].textContent).toContain("N2.2");
    expect(rows[0].textContent).toContain("Regime forfettario o minimi");
  });

  it("supports keyboard navigation (ArrowDown, ArrowUp, Enter, Escape) and backdrop close", () => {
    const fixture = TestBed.createComponent(DocSelectComponent);
    fixture.componentRef.setInput("options", NATURA_OPTIONS.slice(0, 3));
    const emitted: string[] = [];
    const sub = fixture.componentInstance.valueChange.subscribe((val: string) => emitted.push(val));
    fixture.detectChanges();

    const hostEl = fixture.nativeElement as HTMLElement;
    hostEl.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true }));
    fixture.detectChanges();
    expect(fixture.componentInstance.isOpen()).toBe(true);
    expect(fixture.componentInstance.activeIndex()).toBe(0);

    hostEl.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true }));
    fixture.detectChanges();
    expect(fixture.componentInstance.activeIndex()).toBe(1);

    hostEl.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
    fixture.detectChanges();
    expect(emitted).toEqual(["N2.1"]);
    expect(fixture.componentInstance.isOpen()).toBe(false);

    fixture.componentInstance.openDropdown();
    fixture.detectChanges();
    const backdrop = hostEl.querySelector<HTMLElement>(".select-backdrop")!;
    backdrop.click();
    fixture.detectChanges();
    expect(fixture.componentInstance.isOpen()).toBe(false);
    sub.unsubscribe();
  });

  it("integrates with Reactive Forms via ControlValueAccessor, clear button, and disabled state", () => {
    const fixture = TestBed.createComponent(TestHostFormComponent);
    fixture.detectChanges();

    const host = fixture.componentInstance;
    const el: HTMLElement = fixture.nativeElement;
    expect(el.querySelector(".doc-code-badge")?.textContent?.trim()).toBe("N2.2");

    const clearBtn: HTMLButtonElement = el.querySelector(".doc-clear-btn")!;
    expect(clearBtn).not.toBeNull();
    clearBtn.click();
    fixture.detectChanges();
    expect(host.control.value).toBe("");

    const trigger: HTMLButtonElement = el.querySelector(".doc-select-trigger")!;
    trigger.click();
    fixture.detectChanges();
    const optionButtons = el.querySelectorAll<HTMLButtonElement>(".doc-option-row:not(.doc-option-empty)");
    optionButtons[4].click();
    fixture.detectChanges();
    expect(host.control.value).toBe("N4");

    host.control.disable();
    fixture.detectChanges();
    expect(trigger.disabled).toBe(true);
    expect(el.querySelector(".doc-clear-btn")).toBeNull();
  });
});
