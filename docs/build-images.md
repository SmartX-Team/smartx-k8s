# Building container images on local

All container images within a project are built via a `helm template`.
This means that on-demand container images are generated based on `values.yaml` values, just like a regular `helm template` command.
Variables shared by multiple images, such as distribution versions, can be passed from `values.yaml` in the repository root via `patches.yaml`.
This provides a [single source of truth](https://en.wikipedia.org/wiki/Single_source_of_truth), ensuring consistency across the cluster.

## TL;DR

Yon can just build and push an image within [`./images`](/images/) like below:

```bash
just build-image [name]
```

For example, if you want to build a [`kubespray`](/images/kubespray/) image,

```bash
just build-image kubespray
```
