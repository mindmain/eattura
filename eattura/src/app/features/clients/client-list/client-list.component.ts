import { ChangeDetectionStrategy, Component, computed, inject, OnInit, signal } from "@angular/core";
import { Router, RouterLink } from "@angular/router";
import { ClientStore } from "../../../core/stores/client.store";
import { NotificationService } from "../../../core/services/notification.service";
import { ClientNamePipe } from "../../../shared/pipes/client-name.pipe";

@Component({
  selector: "app-client-list",
  standalone: true,
  imports: [RouterLink, ClientNamePipe],
  templateUrl: "./client-list.component.html",
  styleUrl: "./client-list.component.css",
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ClientListComponent implements OnInit {
  readonly clientStore = inject(ClientStore);
  private router = inject(Router);
  private notify = inject(NotificationService);

  readonly clients = this.clientStore.clients;
  readonly loading = this.clientStore.loading;
  searchTerm = signal("");

  filteredClients = computed(() => {
    const term = this.searchTerm().trim().toLowerCase();
    if (!term) return this.clients();
    return this.clients().filter((c) => {
      const text = c.searchText ?? [c.denominazione, c.nome, c.cognome, c.idCodice, c.comune]
        .filter(Boolean).join(" ").toLowerCase();
      return term.split(/\s+/).every((part) => text.includes(part));
    });
  });

  async ngOnInit(): Promise<void> {
    try {
      await this.clientStore.loadAll();
    } catch {
      this.notify.error("Errore nel caricamento dei clienti");
    }
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
    if (typeof window !== "undefined" && !window.confirm("Sei sicuro di voler eliminare questo cliente?")) return;
    try {
      await this.clientStore.deleteClient(id);
      this.notify.success("Cliente eliminato");
    } catch {
      this.notify.error("Errore durante l'eliminazione del cliente");
    }
  }
}
