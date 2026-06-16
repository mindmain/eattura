import { Component, computed, inject, OnInit, signal } from "@angular/core";
import { Router, RouterLink } from "@angular/router";
import { ClientService } from "../../../core/services/client.service";
import { NotificationService } from "../../../core/services/notification.service";
import type { ClientSummary } from "../../../core/models/client.model";

@Component({
  selector: "app-client-list",
  standalone: true,
  imports: [RouterLink],
  templateUrl: "./client-list.component.html",
  styleUrl: "./client-list.component.css",
})
export class ClientListComponent implements OnInit {
  private clientService = inject(ClientService);
  private router = inject(Router);
  private notify = inject(NotificationService);

  clients = signal<ClientSummary[]>([]);
  searchTerm = signal("");

  filteredClients = computed(() => {
    const term = this.searchTerm().toLowerCase();
    if (!term) return this.clients();
    return this.clients().filter((c) => {
      const name = c.denominazione ?? `${c.nome ?? ""} ${c.cognome ?? ""}`;
      return (
        name.toLowerCase().includes(term) ||
        c.idCodice.toLowerCase().includes(term) ||
        c.comune.toLowerCase().includes(term)
      );
    });
  });

  async ngOnInit(): Promise<void> {
    await this.loadClients();
  }

  onSearch(event: Event): void {
    const input = event.target as HTMLInputElement;
    this.searchTerm.set(input.value);
  }

  navigateToNew(): void {
    this.router.navigate(["/clients/new"]);
  }

  navigateToEdit(id: string): void {
    this.router.navigate(["/clients", id, "edit"]);
  }

  async deleteClient(id: string): Promise<void> {
    if (!confirm("Sei sicuro di voler eliminare questo cliente?")) return;
    try {
      await this.clientService.deleteClient(id);
      this.notify.success("Cliente eliminato");
      await this.loadClients();
    } catch (e) {
      this.notify.error("Errore durante l'eliminazione del cliente");
    }
  }

  displayName(client: ClientSummary): string {
    return client.denominazione ?? `${client.nome ?? ""} ${client.cognome ?? ""}`.trim();
  }

  private async loadClients(): Promise<void> {
    try {
      const list = await this.clientService.listClients();
      this.clients.set(list);
    } catch {
      this.notify.error("Errore nel caricamento dei clienti");
    }
  }
}
