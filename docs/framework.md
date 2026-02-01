# Framework Structure and Key Components

## App of Apps Pattern

- Manages multiple Argo CD applications using Helm
- Uses a layered GitOps structure with centralized control
- Supports modular configuration via reusable preset repositories

## [OpenARK](/docs/openark.md) Components

- **KISS**: A simplified Kubernetes configuration for edge clusters
- **Spectrum API**: A `Histogram API` to classify the services by metrics and a `Pool API` to provisioning the services without handling containers, compatible with `Gateway API`
- **VINE Dashboard**: A web-based UI for managing resources and sessions
- **VINE Session Operator**: Desktop-as-a-Service implementation over containers and VMs
