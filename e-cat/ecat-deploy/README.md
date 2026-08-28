# e-cat Deploy

Deployment templates for the e-cat ecosystem.

## Contents

| Path | Purpose |
|------|---------|
| `Dockerfile` | Multi-stage Rust build → minimal runtime image |
| `k8s-deployment.yaml` | Kubernetes Deployment + Service manifest |
| `helm/` | Helm Chart for templated deployments |

## Usage

```bash
# Docker
docker build -t ecat-app -f ecat-deploy/Dockerfile .

# Kubernetes
kubectl apply -f ecat-deploy/k8s-deployment.yaml

# Helm
helm install my-ecat ecat-deploy/helm/
```
