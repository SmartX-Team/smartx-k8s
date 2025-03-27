#!/usr/bin/env python

from typing import Any

import networkx as nx
import pydantic
import yaml


class Feature(pydantic.BaseModel):
    requires: list[str] = []
    optional: list[str] = []
    provides: list[str] = []


def build_graph(base_dir: str) -> nx.Graph:
    # Load data
    fp = open(f'{base_dir}/apps/template/features.yaml')
    data: dict[str, Any] = yaml.safe_load(fp)

    # Load features
    features = {
        name: Feature.model_validate(item)
        for (name, item) in data.items()
    }

    # Collect into a graph
    g = nx.DiGraph()
    for node in features.keys():
        g.add_node(node)
    for dst, feature in features.items():
        for src in feature.requires:
            g.add_edge(src, dst)
        # for src in feature.optional:
        #     g.add_edge(src, dst)
        # for src in feature.provides:
        #     g.add_edge(dst, src)
    return g


if __name__ == '__main__':
    print(build_graph('.'))
