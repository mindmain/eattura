import { Pipe, PipeTransform } from "@angular/core";
import { CONDIZIONI_PAGAMENTO } from "../../core/models/sdi.model";

@Pipe({
  name: "condizioniPagamento",
  standalone: true,
  pure: true,
})
export class CondizioniPagamentoPipe implements PipeTransform {
  transform(code: string | null | undefined): string {
    if (!code) return "";
    return CONDIZIONI_PAGAMENTO[code] ?? code;
  }
}
