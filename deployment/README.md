# deployment

Deployment configuration for Shadowmask. The container image is built from the `Dockerfile` at
the repository root.

- [`dev`](./dev) - builds and runs the server locally for development and testing; the
  end-to-end smoke test runs against this stack.
- [`production`](./production) - a template for running the published image in production.
