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
import { Calendar, ChevronLeft, ChevronRight, LucideAngularModule, X } from "lucide-angular";

export interface CalendarDayCell {
  iso: string;
  day: number;
  inCurrentMonth: boolean;
  isToday: boolean;
  isSelected: boolean;
  isDisabled: boolean;
}

interface ParsedDateParts {
  year: number;
  month: number; // 0-based (0 = January .. 11 = December)
  day: number;
}

const ISO_DATE_RE = /^(\d{4})-(\d{2})-(\d{2})$/;

export const ITALIAN_MONTHS: readonly string[] = [
  "Gennaio",
  "Febbraio",
  "Marzo",
  "Aprile",
  "Maggio",
  "Giugno",
  "Luglio",
  "Agosto",
  "Settembre",
  "Ottobre",
  "Novembre",
  "Dicembre",
];

export const ITALIAN_WEEKDAYS: readonly string[] = [
  "Lun",
  "Mar",
  "Mer",
  "Gio",
  "Ven",
  "Sab",
  "Dom",
];

function parseIsoDate(iso: string | null | undefined): ParsedDateParts | null {
  if (!iso) {
    return null;
  }
  const match = ISO_DATE_RE.exec(iso.trim());
  if (!match) {
    return null;
  }
  const year = Number(match[1]);
  const month = Number(match[2]) - 1;
  const day = Number(match[3]);
  const probe = new Date(year, month, day);
  if (
    probe.getFullYear() !== year ||
    probe.getMonth() !== month ||
    probe.getDate() !== day
  ) {
    return null;
  }
  return { year, month, day };
}

function formatIsoDate(year: number, month: number, day: number): string {
  const y = String(year).padStart(4, "0");
  const m = String(month + 1).padStart(2, "0");
  const d = String(day).padStart(2, "0");
  return `${y}-${m}-${d}`;
}

@Component({
  selector: "app-date-picker",
  standalone: true,
  imports: [LucideAngularModule],
  templateUrl: "./date-picker.component.html",
  styleUrl: "./date-picker.component.css",
  changeDetection: ChangeDetectionStrategy.OnPush,
  providers: [
    {
      provide: NG_VALUE_ACCESSOR,
      useExisting: forwardRef(() => DatePickerComponent),
      multi: true,
    },
  ],
  host: {
    "(keydown)": "onHostKeydown($event)",
  },
})
export class DatePickerComponent implements ControlValueAccessor {
  // JIT signal input/output metadata for Vitest (esbuild without Angular JIT transform); ignored in AOT.
  static readonly __prop__metadata__ = {
    placeholder: [{ ngMetadataName: "Input", isSignal: true }],
    clearable: [{ ngMetadataName: "Input", isSignal: true }],
    minDate: [{ ngMetadataName: "Input", isSignal: true }],
    maxDate: [{ ngMetadataName: "Input", isSignal: true }],
    inputId: [{ ngMetadataName: "Input", isSignal: true }],
    ariaLabel: [{ ngMetadataName: "Input", isSignal: true }],
    value: [{ ngMetadataName: "Input", isSignal: true }],
    disabledInput: [{ ngMetadataName: "Input", isSignal: true, alias: "disabled" }],
    valueChange: [{ ngMetadataName: "Output" }],
  };

  readonly placeholder = input<string>("gg/mm/aaaa");
  readonly clearable = input<boolean>(true);
  readonly minDate = input<string>("");
  readonly maxDate = input<string>("");
  readonly inputId = input<string>("");
  readonly ariaLabel = input<string>("");
  readonly value = input<string | null | undefined>(undefined);
  readonly disabledInput = input<boolean>(false, { alias: "disabled" });

  readonly valueChange = output<string>();

  protected readonly icons = { Calendar, ChevronLeft, ChevronRight, X };
  protected readonly months = ITALIAN_MONTHS;
  protected readonly weekdays = ITALIAN_WEEKDAYS;

  private readonly cvaRegistered = signal<boolean>(false);
  private readonly cvaValue = signal<string>("");
  private readonly cvaDisabled = signal<boolean>(false);
  private readonly syncedInputValue = linkedSignal(() => this.value());

  readonly isOpen = signal<boolean>(false);
  readonly viewYear = signal<number>(new Date().getFullYear());
  readonly viewMonth = signal<number>(new Date().getMonth());

  private onChangeFn: (value: string) => void = () => {};
  private onTouchedFn: () => void = () => {};

  readonly isDisabled = computed(() => this.disabledInput() || this.cvaDisabled());

  readonly currentValue = computed(() => {
    const raw = this.cvaRegistered() ? this.cvaValue() : (this.syncedInputValue() ?? "");
    return parseIsoDate(raw) ? raw.trim() : "";
  });

  readonly formattedValue = computed(() => {
    const parts = parseIsoDate(this.currentValue());
    if (!parts) {
      return "";
    }
    const dd = String(parts.day).padStart(2, "0");
    const mm = String(parts.month + 1).padStart(2, "0");
    const yyyy = String(parts.year).padStart(4, "0");
    return `${dd}/${mm}/${yyyy}`;
  });

  private readonly normalizedMinIso = computed(() =>
    parseIsoDate(this.minDate()) ? this.minDate().trim() : ""
  );

  private readonly normalizedMaxIso = computed(() =>
    parseIsoDate(this.maxDate()) ? this.maxDate().trim() : ""
  );

  readonly yearOptions = computed(() => {
    const center = this.viewYear();
    const years: number[] = [];
    for (let y = center - 15; y <= center + 15; y++) {
      years.push(y);
    }
    return years;
  });

  readonly calendarDays = computed<readonly CalendarDayCell[]>(() => {
    const year = this.viewYear();
    const month = this.viewMonth();
    const selectedIso = this.currentValue();
    const todayIso = this.getTodayIso();
    const minIso = this.normalizedMinIso();
    const maxIso = this.normalizedMaxIso();

    const firstOfMonth = new Date(year, month, 1);
    const mondayOffset = (firstOfMonth.getDay() + 6) % 7;
    const daysInCurrentMonth = new Date(year, month + 1, 0).getDate();
    const daysInPrevMonth = new Date(year, month, 0).getDate();

    const prevMonthYear = month === 0 ? year - 1 : year;
    const prevMonthIdx = month === 0 ? 11 : month - 1;
    const nextMonthYear = month === 11 ? year + 1 : year;
    const nextMonthIdx = month === 11 ? 0 : month + 1;

    const cells: CalendarDayCell[] = [];

    for (let i = mondayOffset - 1; i >= 0; i--) {
      const day = daysInPrevMonth - i;
      const iso = formatIsoDate(prevMonthYear, prevMonthIdx, day);
      cells.push({
        iso,
        day,
        inCurrentMonth: false,
        isToday: iso === todayIso,
        isSelected: iso === selectedIso,
        isDisabled: this.isIsoOutOfRange(iso, minIso, maxIso),
      });
    }

    for (let day = 1; day <= daysInCurrentMonth; day++) {
      const iso = formatIsoDate(year, month, day);
      cells.push({
        iso,
        day,
        inCurrentMonth: true,
        isToday: iso === todayIso,
        isSelected: iso === selectedIso,
        isDisabled: this.isIsoOutOfRange(iso, minIso, maxIso),
      });
    }

    const remaining = 42 - cells.length;
    for (let day = 1; day <= remaining; day++) {
      const iso = formatIsoDate(nextMonthYear, nextMonthIdx, day);
      cells.push({
        iso,
        day,
        inCurrentMonth: false,
        isToday: iso === todayIso,
        isSelected: iso === selectedIso,
        isDisabled: this.isIsoOutOfRange(iso, minIso, maxIso),
      });
    }

    return cells;
  });

  readonly isTodayDisabled = computed(() =>
    this.isIsoOutOfRange(this.getTodayIso(), this.normalizedMinIso(), this.normalizedMaxIso())
  );

  writeValue(value: string | null | undefined): void {
    this.cvaRegistered.set(true);
    this.cvaValue.set(value ?? "");
    const parts = parseIsoDate(value);
    if (parts) {
      this.viewYear.set(parts.year);
      this.viewMonth.set(parts.month);
    }
  }

  registerOnChange(fn: (value: string) => void): void {
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
      this.closePopover();
    } else {
      this.openPopover();
    }
  }

  openPopover(): void {
    if (this.isDisabled() || this.isOpen()) {
      return;
    }
    const selected = parseIsoDate(this.currentValue());
    if (selected) {
      this.viewYear.set(selected.year);
      this.viewMonth.set(selected.month);
    } else {
      const now = new Date();
      this.viewYear.set(now.getFullYear());
      this.viewMonth.set(now.getMonth());
    }
    this.isOpen.set(true);
  }

  closePopover(): void {
    if (!this.isOpen()) {
      return;
    }
    this.isOpen.set(false);
    this.onTouchedFn();
  }

  prevMonth(): void {
    const month = this.viewMonth();
    if (month === 0) {
      this.viewMonth.set(11);
      this.viewYear.update((y) => y - 1);
    } else {
      this.viewMonth.set(month - 1);
    }
  }

  nextMonth(): void {
    const month = this.viewMonth();
    if (month === 11) {
      this.viewMonth.set(0);
      this.viewYear.update((y) => y + 1);
    } else {
      this.viewMonth.set(month + 1);
    }
  }

  onMonthChange(event: Event): void {
    const target = event.target as HTMLSelectElement | null;
    const nextMonth = Number(target?.value);
    if (!Number.isNaN(nextMonth) && nextMonth >= 0 && nextMonth <= 11) {
      this.viewMonth.set(nextMonth);
    }
  }

  onYearChange(event: Event): void {
    const target = event.target as HTMLSelectElement | null;
    const nextYear = Number(target?.value);
    if (!Number.isNaN(nextYear) && nextYear >= 1900 && nextYear <= 2200) {
      this.viewYear.set(nextYear);
    }
  }

  selectDate(cell: CalendarDayCell): void {
    if (this.isDisabled() || cell.isDisabled) {
      return;
    }
    this.applyValue(cell.iso);
    this.closePopover();
  }

  selectToday(): void {
    if (this.isDisabled() || this.isTodayDisabled()) {
      return;
    }
    this.applyValue(this.getTodayIso());
    this.closePopover();
  }

  clearDate(event?: Event): void {
    event?.stopPropagation();
    if (this.isDisabled()) {
      return;
    }
    this.applyValue("");
    if (this.isOpen()) {
      this.closePopover();
    } else {
      this.onTouchedFn();
    }
  }

  onHostKeydown(event: KeyboardEvent): void {
    if (!this.isOpen()) {
      return;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      this.closePopover();
    }
  }

  private applyValue(iso: string): void {
    if (this.cvaRegistered()) {
      this.cvaValue.set(iso);
    } else {
      this.syncedInputValue.set(iso);
    }
    const parts = parseIsoDate(iso);
    if (parts) {
      this.viewYear.set(parts.year);
      this.viewMonth.set(parts.month);
    }
    this.onChangeFn(iso);
    this.valueChange.emit(iso);
  }

  private getTodayIso(): string {
    const now = new Date();
    return formatIsoDate(now.getFullYear(), now.getMonth(), now.getDate());
  }

  private isIsoOutOfRange(iso: string, minIso: string, maxIso: string): boolean {
    if (minIso && iso < minIso) {
      return true;
    }
    if (maxIso && iso > maxIso) {
      return true;
    }
    return false;
  }
}
