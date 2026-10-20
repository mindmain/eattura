import {
  ApplicationConfig,
  ErrorHandler,
  provideBrowserGlobalErrorListeners,
  provideZonelessChangeDetection,
} from "@angular/core";
import { provideRouter, withComponentInputBinding } from "@angular/router";

import { routes } from "./app.routes";
import { AppErrorHandler } from "./core/services/app-log.service";

export const appConfig: ApplicationConfig = {
  providers: [
    { provide: ErrorHandler, useClass: AppErrorHandler },
    provideBrowserGlobalErrorListeners(),
    provideZonelessChangeDetection(),
    provideRouter(routes, withComponentInputBinding()),
  ],
};
