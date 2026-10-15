import { Pipe, PipeTransform } from "@angular/core";

@Pipe({
  name: "appCurrency",
  standalone: true,
  pure: true,
})
export class AppCurrencyPipe implements PipeTransform {
  private formatter = new Intl.NumberFormat("it-IT", {
    style: "currency",
    currency: "EUR",
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  });

  transform(value: number | null | undefined): string {
    if (value == null || Number.isNaN(Number(value))) return "-";
    return this.formatter.format(Number(value));
  }
}
