# Angular app

- [core](core/AGENTS.md): DTOs, IPC services and shared stores.
- [features](features/AGENTS.md): routed screens.
- [shared](shared/AGENTS.md): reusable UI and pipes.

Register routes in `app.routes.ts`. Use standalone OnPush components and signal state. Keep desktop calls behind core services and preserve the macOS title bar controls.
