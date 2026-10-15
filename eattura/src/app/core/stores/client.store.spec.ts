import { describe, it, expect, vi, beforeEach } from "vitest";
import { Injector } from "@angular/core";
import { ClientStore } from "./client.store";
import { ClientService } from "../services/client.service";
import type { ClientSummary, ClientDetail, CreateClientRequest } from "../models/client.model";

describe("ClientStore", () => {
  const mockClients: ClientSummary[] = [
    {
      id: "client-1",
      denominazione: "Acme Corp",
      nome: null,
      cognome: null,
      idPaese: "IT",
      idCodice: "12345678901",
      comune: "Roma",
    },
    {
      id: "client-2",
      denominazione: "Beta Ltd",
      nome: null,
      cognome: null,
      idPaese: "IT",
      idCodice: "98765432109",
      comune: "Milano",
    },
  ];

  let mockClientService: {
    listClients: ReturnType<typeof vi.fn>;
    getClient: ReturnType<typeof vi.fn>;
    createClient: ReturnType<typeof vi.fn>;
    updateClient: ReturnType<typeof vi.fn>;
    deleteClient: ReturnType<typeof vi.fn>;
  };
  let store: ClientStore;

  beforeEach(() => {
    mockClientService = {
      listClients: vi.fn().mockResolvedValue(mockClients),
      getClient: vi.fn(),
      createClient: vi.fn(),
      updateClient: vi.fn(),
      deleteClient: vi.fn(),
    };

    const injector = Injector.create({
      providers: [
        ClientStore,
        { provide: ClientService, useValue: mockClientService },
      ],
    });

    store = injector.get(ClientStore);
  });

  it("should initialize empty signals", () => {
    expect(store.clients()).toEqual([]);
    expect(store.loading()).toBe(false);
    expect(store.clientCount()).toBe(0);
    expect(store.clientMap().size).toBe(0);
  });

  it("should load clients and compute clientMap & clientCount", async () => {
    await store.loadAll();

    expect(store.clients()).toHaveLength(2);
    expect(store.clientCount()).toBe(2);
    expect(store.clientMap().get("client-1")?.denominazione).toBe("Acme Corp");
  });

  it("should remove deleted client from state without refetching", async () => {
    await store.loadAll();
    mockClientService.deleteClient.mockResolvedValue(undefined);

    await store.deleteClient("client-1");

    expect(store.clients()).toHaveLength(1);
    expect(store.clientMap().has("client-1")).toBe(false);
    expect(store.clientCount()).toBe(1);
  });
});
