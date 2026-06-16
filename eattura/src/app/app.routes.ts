import { Routes } from "@angular/router";

export const routes: Routes = [
  { path: "", redirectTo: "dashboard", pathMatch: "full" },
  {
    path: "dashboard",
    loadComponent: () =>
      import("./features/dashboard/dashboard.component").then(
        (m) => m.DashboardComponent
      ),
  },
  {
    path: "clients",
    loadComponent: () =>
      import("./features/clients/client-list/client-list.component").then(
        (m) => m.ClientListComponent
      ),
  },
  {
    path: "clients/new",
    loadComponent: () =>
      import("./features/clients/client-form/client-form.component").then(
        (m) => m.ClientFormComponent
      ),
  },
  {
    path: "clients/:id/edit",
    loadComponent: () =>
      import("./features/clients/client-form/client-form.component").then(
        (m) => m.ClientFormComponent
      ),
  },
  {
    path: "invoices",
    loadComponent: () =>
      import("./features/invoices/invoice-list/invoice-list.component").then(
        (m) => m.InvoiceListComponent
      ),
  },
  {
    path: "invoices/new",
    loadComponent: () =>
      import("./features/invoices/invoice-form/invoice-form.component").then(
        (m) => m.InvoiceFormComponent
      ),
  },
  {
    path: "invoices/:id",
    loadComponent: () =>
      import(
        "./features/invoices/invoice-detail/invoice-detail.component"
      ).then((m) => m.InvoiceDetailComponent),
  },
  {
    path: "invoices/:id/edit",
    loadComponent: () =>
      import("./features/invoices/invoice-form/invoice-form.component").then(
        (m) => m.InvoiceFormComponent
      ),
  },
  {
    path: "settings",
    loadComponent: () =>
      import("./features/settings/settings.component").then(
        (m) => m.SettingsComponent
      ),
  },
];
