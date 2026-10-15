import { Pipe, PipeTransform } from "@angular/core";
import type { InvoiceStatus } from "../../core/models/invoice.model";

const LABELS: Record<InvoiceStatus, string> = {
  draft: "Bozza",
  validated: "Validata",
  sent: "Inviata",
  accepted: "Accettata",
  rejected: "Rifiutata",
};

@Pipe({
  name: "invoiceStatus",
  standalone: true,
  pure: true,
})
export class InvoiceStatusPipe implements PipeTransform {
  transform(status: InvoiceStatus | string | null | undefined): string {
    if (!status) return "";
    return LABELS[status as InvoiceStatus] ?? status;
  }
}
