import { Pipe, PipeTransform } from "@angular/core";
import { TIPO_DOCUMENTO } from "../../core/models/sdi.model";

@Pipe({
  name: "tipoDocumento",
  standalone: true,
  pure: true,
})
export class TipoDocumentoPipe implements PipeTransform {
  transform(code: string | null | undefined): string {
    if (!code) return "";
    return TIPO_DOCUMENTO[code] ?? code;
  }
}
