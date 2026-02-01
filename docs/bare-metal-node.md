# How to control the bare-metal nodes manually?

## TL;DR

### List all nodes

```bash
# This will show the nodes' name, alias, cluster role, state and timestamps
just box
```

### SSH

```bash
just ssh [name] *CMD
```

### SSH (Batch; sequential)

```bash
just batch COMMAND *ARGS
```
