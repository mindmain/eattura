import { Injectable, computed, inject, signal } from "@angular/core";
import { ClientService } from "../services/client.service";
import type {
  ClientSummary,
  ClientDetail,
  CreateClientRequest,
} from "../models/client.model";

/**
 * Reactive Signal Store for Clients.
 *
 * Centralizes client state in memory to prevent redundant IPC queries
 * across navigation between Dashboard, Invoices, and Client directory.
 */
@Injectable({ providedIn: "root" })
export class ClientStore {
  private clientService = inject(ClientService);

  private _clients = signal<ClientSummary[]>([]);
  private _loading = signal<boolean>(false);

  // Readonly signals exposed to components
  readonly clients = this._clients.asReadonly();
  readonly loading = this._loading.asReadonly();
  readonly clientCount = computed(() => this._clients().length);

  /** Map of clients by ID for fast synchronous lookups. */
  readonly clientMap = computed(() => {
    const map = new Map<string, ClientSummary>();
    for (const client of this._clients()) {
      map.set(client.id, client);
    }
    return map;
  });

  /**
   * Load all clients into store.
   * If already loaded and force is false, reuses in-memory cache.
   */
  async loadAll(force = false): Promise<ClientSummary[]> {
    if (this._clients().length > 0 && !force) {
      return this._clients();
    }
    this._loading.set(true);
    try {
      const list = await this.clientService.listClients();
      this._clients.set(list);
      return list;
    } finally {
      this._loading.set(false);
    }
  }

  /** Force reload clients from backend. */
  async refresh(): Promise<ClientSummary[]> {
    return this.loadAll(true);
  }

  /** Fetch a single client detail by ID. */
  async getClient(id: string): Promise<ClientDetail> {
    return this.clientService.getClient(id);
  }

  /** Create a new client and prepend it to the in-memory list. */
  async createClient(request: CreateClientRequest): Promise<ClientDetail> {
    const detail = await this.clientService.createClient(request);
    await this.refresh();
    return detail;
  }

  /** Update an existing client and update the in-memory list. */
  async updateClient(id: string, request: CreateClientRequest): Promise<ClientDetail> {
    const detail = await this.clientService.updateClient(id, request);
    await this.refresh();
    return detail;
  }

  /** Delete a client and remove it from the in-memory list. */
  async deleteClient(id: string): Promise<void> {
    await this.clientService.deleteClient(id);
    this._clients.update((list) => list.filter((c) => c.id !== id));
  }
}
