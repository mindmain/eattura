import { Injectable } from "@angular/core";
import { TauriService } from "./tauri.service";
import type {
  ClientSummary,
  ClientDetail,
  CreateClientRequest,
} from "../models/client.model";

/**
 * Service for client operations via Tauri IPC.
 * All methods call Rust backend commands defined in commands/clients.rs.
 */
@Injectable({ providedIn: "root" })
export class ClientService {
  constructor(private tauri: TauriService) {}

  listClients(): Promise<ClientSummary[]> {
    return this.tauri.invoke("list_clients");
  }

  getClient(id: string): Promise<ClientDetail> {
    return this.tauri.invoke("get_client", { id });
  }

  createClient(client: CreateClientRequest): Promise<ClientDetail> {
    return this.tauri.invoke("create_client", { client });
  }

  updateClient(id: string, client: CreateClientRequest): Promise<ClientDetail> {
    return this.tauri.invoke("update_client", { id, client });
  }

  deleteClient(id: string): Promise<void> {
    return this.tauri.invoke("delete_client", { id });
  }
}
