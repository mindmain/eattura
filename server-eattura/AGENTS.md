# HTTP server

Optional stateless Axum service. [src](src/AGENTS.md) implements PEC, SdI tools and VIES. [tests](tests/AGENTS.md) exercises the API. Deployments manage HTTP access control. Keep requests transient and storage out of the server.

From the root, use `task server` or `cargo test --locked -p server-eattura`.
