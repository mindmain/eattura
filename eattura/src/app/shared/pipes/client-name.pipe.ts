import { Pipe, PipeTransform } from "@angular/core";
import type { ClientSummary } from "../../core/models/client.model";

@Pipe({
  name: "clientName",
  standalone: true,
  pure: true,
})
export class ClientNamePipe implements PipeTransform {
  transform(client: ClientSummary | null | undefined): string {
    if (!client) return "";
    return client.denominazione ?? `${client.nome ?? ""} ${client.cognome ?? ""}`.trim();
  }
}
