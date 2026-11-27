import {
  ChangeDetectionStrategy,
  Component,
  computed,
  forwardRef,
  input,
  linkedSignal,
  output,
  signal,
} from "@angular/core";
import { ControlValueAccessor, NG_VALUE_ACCESSOR } from "@angular/forms";
import { ChevronDown, LucideAngularModule, Search, X } from "lucide-angular";

export interface DocSelectOption<T extends string | number = string> {
  value: T;
  code: string;
  label: string;
  detail?: string;
}

@Component({
  selector: "app-doc-select",
  standalone: true,
  imports: [LucideAngularModule],
  templateUrl: "./doc-select.component.html",
  styleUrl: "./doc-select.component.css",
  changeDetection: ChangeDetectionStrategy.OnPush,
  providers: [
    {
      provide: NG_VALUE_ACCESSOR,
      useExisting: forwardRef(() => DocSelectComponent),
      multi: true,
    },
  ],
  host: {
    "(keydown)": "onHostKeydown($event)",
  },
})
export class DocSelectComponent implements ControlValueAccessor {
  // JIT signal input/output metadata for Vitest (esbuild without Angular JIT transform); ignored in AOT.
  static readonly __prop__metadata__ = {
    options: [{ ngMetadataName: "Input", isSignal: true, required: true }],
    placeholder: [{ ngMetadataName: "Input", isSignal: true }],
    emptyLabel: [{ ngMetadataName: "Input", isSignal: true }],
    clearable: [{ ngMetadataName: "Input", isSignal: true }],
    searchable: [{ ngMetadataName: "Input", isSignal: true }],
    inputId: [{ ngMetadataName: "Input", isSignal: true }],
    ariaLabel: [{ ngMetadataName: "Input", isSignal: true }],
    value: [{ ngMetadataName: "Input", isSignal: true }],
    disabledInput: [{ ngMetadataName: "Input", isSignal: true, alias: "disabled" }],
    valueChange: [{ ngMetadataName: "Output" }],
  };

  readonly options = input.required<readonly DocSelectOption<any>[]>();
  readonly placeholder = input<string>("Seleziona…");
  readonly emptyLabel = input<string>("");
  readonly clearable = input<boolean>(false);
  readonly searchable = input<boolean | null>(null);
  readonly inputId = input<string>("");
  readonly ariaLabel = input<string>("");
  readonly value = input<string | number | null | undefined>(undefined);
  readonly disabledInput = input<boolean>(false, { alias: "disabled" });

  readonly valueChange = output<any>();

  protected readonly icons = { ChevronDown, Search, X };

  private readonly cvaRegistered = signal<boolean>(false);
  private readonly cvaValue = signal<string | number | null>(null);
  private readonly cvaDisabled = signal<boolean>(false);
  private readonly syncedInputValue = linkedSignal(() => this.value());

  readonly isOpen = signal<boolean>(false);
  readonly searchQuery = signal<string>("");
  readonly activeIndex = signal<number>(-1);

  private onChangeFn: (value: any) => void = () => {};
  private onTouchedFn: () => void = () => {};

  readonly isDisabled = computed(() => this.disabledInput() || this.cvaDisabled());

  readonly currentValue = computed(() =>
    this.cvaRegistered() ? this.cvaValue() : (this.syncedInputValue() ?? null)
  );

  readonly selectedOption = computed(() => {
    const val = this.currentValue();
    if (val === null || val === undefined) {
      return undefined;
    }
    return this.options().find((opt) => opt.value === val);
  });

  readonly hasSelectedValue = computed(() => this.selectedOption() !== undefined);

  readonly isSearchable = computed(() => {
    const explicit = this.searchable();
    return explicit !== null ? explicit : this.options().length > 6;
  });

  readonly showEmptyOption = computed(() => {
    const label = this.emptyLabel().trim();
    if (!label) {
      return false;
    }
    const query = this.searchQuery().trim().toLowerCase();
    if (!query) {
      return true;
    }
    return label.toLowerCase().includes(query);
  });

  readonly filteredOptions = computed(() => {
    const all = this.options();
    const query = this.searchQuery().trim().toLowerCase();
    if (!query) {
      return all;
    }
    return all.filter(
      (opt) =>
        opt.code.toLowerCase().includes(query) ||
        opt.label.toLowerCase().includes(query) ||
        (opt.detail?.toLowerCase().includes(query) ?? false)
    );
  });

  writeValue(value: any): void {
    this.cvaRegistered.set(true);
    this.cvaValue.set(value ?? null);
  }

  registerOnChange(fn: (value: any) => void): void {
    this.cvaRegistered.set(true);
    this.onChangeFn = fn;
  }

  registerOnTouched(fn: () => void): void {
    this.onTouchedFn = fn;
  }

  setDisabledState(isDisabled: boolean): void {
    this.cvaDisabled.set(isDisabled);
    if (isDisabled && this.isOpen()) {
      this.isOpen.set(false);
    }
  }

  toggleOpen(): void {
    if (this.isDisabled()) {
      return;
    }
    if (this.isOpen()) {
      this.closeDropdown();
    } else {
      this.openDropdown();
    }
  }

  openDropdown(): void {
    if (this.isDisabled() || this.isOpen()) {
      return;
    }
    this.searchQuery.set("");
    this.isOpen.set(true);
    const current = this.currentValue();
    const list = this.filteredOptions();
    const idx = list.findIndex((opt) => opt.value === current);
    this.activeIndex.set(idx >= 0 ? idx : list.length > 0 ? 0 : -1);
  }

  closeDropdown(): void {
    if (!this.isOpen()) {
      return;
    }
    this.isOpen.set(false);
    this.searchQuery.set("");
    this.activeIndex.set(-1);
    this.onTouchedFn();
  }

  onSearchInput(event: Event): void {
    const target = event.target as HTMLInputElement | null;
    this.searchQuery.set(target?.value ?? "");
    const list = this.filteredOptions();
    this.activeIndex.set(list.length > 0 ? 0 : -1);
  }

  selectOption(option: DocSelectOption<any>): void {
    if (this.isDisabled()) {
      return;
    }
    this.applyValue(option.value);
    this.closeDropdown();
  }

  selectEmpty(event?: Event): void {
    event?.stopPropagation();
    if (this.isDisabled()) {
      return;
    }
    this.applyValue("");
    this.closeDropdown();
  }

  clearSelection(event: Event): void {
    event.stopPropagation();
    if (this.isDisabled()) {
      return;
    }
    this.applyValue("");
    this.onTouchedFn();
  }

  isOptionSelected(option: DocSelectOption<any>): boolean {
    return this.currentValue() === option.value;
  }

  onHostKeydown(event: KeyboardEvent): void {
    if (this.isDisabled()) {
      return;
    }

    if (!this.isOpen()) {
      if (event.key === "ArrowDown" || event.key === "ArrowUp") {
        event.preventDefault();
        this.openDropdown();
      }
      return;
    }

    const list = this.filteredOptions();
    switch (event.key) {
      case "Escape": {
        event.preventDefault();
        event.stopPropagation();
        this.closeDropdown();
        break;
      }
      case "ArrowDown": {
        event.preventDefault();
        if (list.length > 0) {
          const next = (this.activeIndex() + 1) % list.length;
          this.activeIndex.set(next);
        }
        break;
      }
      case "ArrowUp": {
        event.preventDefault();
        if (list.length > 0) {
          const current = this.activeIndex();
          const prev = current <= 0 ? list.length - 1 : current - 1;
          this.activeIndex.set(prev);
        }
        break;
      }
      case "Enter": {
        event.preventDefault();
        const idx = this.activeIndex();
        if (idx >= 0 && idx < list.length) {
          this.selectOption(list[idx]);
        }
        break;
      }
      case "Tab": {
        this.closeDropdown();
        break;
      }
    }
  }

  private applyValue(nextValue: any): void {
    if (this.cvaRegistered()) {
      this.cvaValue.set(nextValue);
    } else {
      this.syncedInputValue.set(nextValue);
    }
    this.onChangeFn(nextValue);
    this.valueChange.emit(nextValue);
  }
}
