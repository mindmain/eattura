import { Pipe, PipeTransform } from "@angular/core";
import { MODALITA_PAGAMENTO } from "../../core/models/sdi.model";

@Pipe({
  name: "modalitaPagamento",
  standalone: true,
  pure: true,
})
export class ModalitaPagamentoPipe implements PipeTransform {
  transform(code: string | null | undefined): string {
    if (!code) return "";
    return MODALITA_PAGAMENTO[code] ?? code;
  }
}
