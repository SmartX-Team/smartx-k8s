use std::{collections::BTreeMap, ops};

use anyhow::{Result, anyhow, bail};
use itertools::Itertools;
use pastey::paste;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

use crate::{
    builder::Builder,
    format::DynFormat,
    parser::Parser,
    script::{Edge, Node, NodeMetadata, Script, TypedEdge},
    sink::Sink,
    src::Source,
    store::DynStore,
    table::DynTable,
};

macro_rules! define_context {
    (
        components: {
            $(
                $name:ident : $ty:ty ,
            )*
        },
    ) => {
        paste! {
            #[derive(
                Copy,
                Clone,
                Debug,
                Display,
                EnumString,
                PartialEq,
                Eq,
                Hash,
                Serialize,
                Deserialize,
            )]
            #[serde(rename_all = "PascalCase")]
            pub enum Kind {
                $(
                    [< $name:upper_camel >],
                )*
            }

            #[derive(Default)]
            pub struct CompiledScript {
                $(
                    pub [< "nodes_" $name >]: BTreeMap<String, $ty>,
                )*
                pub edges: Vec<TypedEdge>,
            }

            #[derive(Default)]
            pub struct Context {
                $(
                    [< "builder_" $name >]: BTreeMap<String, Box<dyn Builder<$ty>>>,
                )*
            }

            impl Context {
                $(
                    #[inline]
                    pub fn [< "register_" $name >](&mut self, builder: impl 'static + Builder<$ty>) {
                        self.[< "register_" $name "_dyn" >](Box::new(builder))
                    }

                    #[inline]
                    fn [< "register_" $name "_dyn" >](&mut self, builder: Box<dyn Builder<$ty>>) {
                        let name = builder.name();

                        if builder.kind() != Kind::[< $name:upper_camel >] {
                            #[cfg(feature = "tracing")]
                            {
                                ::tracing::error!("Invalid {}: {}", stringify!($name), name);
                                return;
                            }

                            #[cfg(not(feature = "tracing"))]
                            {
                                panic!("Invalid {}: {}", stringify!($name), name);
                            }
                        }

                        self.[< "builder_" $name >].insert(name, builder);
                    }
                )*

                fn build(&mut self, script: Script) -> Result<CompiledScript> {
                    let Script { nodes, edges } = script;
                    let mut compiled = CompiledScript::default();

                    // Parse edges
                    let convert_edge = |name: &str| {
                        let nodes: Vec<_> = nodes
                            .iter()
                            .filter(|&node| node.metadata.name == name)
                            .map(|node| &node.metadata)
                            .collect();
                        match nodes.as_slice() {
                            &[] => bail!("No such node: {name}"),
                            &[node] => Ok(node.clone()),
                            nodes => bail!(
                                "Multiple nodes are found: {}",
                                nodes.iter().map(ToString::to_string).join(", ")
                            ),
                        }
                    };
                    for Edge { src, sink } in edges {
                        compiled.edges.push(TypedEdge {
                            src: convert_edge(&src)?,
                            sink: convert_edge(&sink)?,
                        });
                    }

                    // Parse nodes
                    for Node { metadata, params } in nodes {
                        let NodeMetadata { kind, model, name } = &metadata;
                        match *kind {
                            $(
                                Kind::[< $name:upper_camel >] => {
                                    let builder = self
                                        .[< "builder_" $name >]
                                        .get_mut(model)
                                        .ok_or_else(|| anyhow!("No such {}: {model}", stringify!($name)))?;
                                    let value = builder.build(params)?;
                                    if compiled.[< "nodes_" $name >].insert(name.clone(), value).is_some() {
                                        bail!("Multiple nodes are found: {metadata}");
                                    }
                                }
                            )*
                        }
                    }

                    Ok(compiled)
                }
            }
        }
    };
}

define_context!(
    components: {
        format: DynFormat,
        sink: Sink,
        source: Source,
        store: DynStore,
        table: DynTable,
    },
);

pub trait Kernel {
    fn wait(&mut self, script: CompiledScript) -> Result<()>;
}

#[derive(Default)]
pub struct DarkLake<R> {
    context: Context,
    parser: Parser,
    runtime: R,
}

impl<R> ops::Deref for DarkLake<R> {
    type Target = Context;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.context
    }
}

impl<R> ops::DerefMut for DarkLake<R> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.context
    }
}

impl<R> DarkLake<R>
where
    R: Kernel,
{
    #[inline]
    pub fn parse(&self, expr: &str) -> Result<Script> {
        self.parser.parse(expr)
    }

    #[inline]
    pub fn wait(self, vm: Script) -> Result<()> {
        let Self {
            mut context,
            parser,
            mut runtime,
        } = self;

        let vmi = context.build(vm)?;
        drop(context);
        drop(parser);

        runtime.wait(vmi)
    }
}
