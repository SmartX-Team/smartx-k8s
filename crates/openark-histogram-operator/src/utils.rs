use anyhow::{Result, anyhow, bail};
use k8s_openapi::api::core::v1::Service;
use kube::{Resource, ResourceExt};
use openark_histogram_api::{
    common::{ObjectReference, ServiceReference},
    histogram::HistogramSettings,
    histogram_class::{HistogramClassCrd, HistogramClassSpec},
};
use ordered_float::{Float, OrderedFloat};
use url::Url;

pub(crate) struct Histogram<T> {
    pub(crate) data: Vec<Vec<T>>,
}

impl<T> Histogram<T> {
    pub(crate) fn build(
        settings: &HistogramSettings,
        data: &[T],
        weights: Vec<Option<OrderedFloat<f64>>>,
    ) -> Result<Self>
    where
        T: Clone,
    {
        const DEFAULT_WEIGHT: u64 = (u64::MIN + u64::MAX) / 2;

        let HistogramSettings {
            accumulate,
            interval: _,
            size,
        } = *settings;

        let accumulate = accumulate.unwrap_or_default();

        // Validate data
        assert_eq!(data.len(), weights.len());

        // Validate size
        if size == 0 {
            bail!("Invalid histogram size: Too small")
        } else if size as u32 > u64::BITS - 1 {
            bail!("Invalid histogram size: Too large")
        }

        // Find min-max
        let (min, max) = weights
            .iter()
            .copied()
            .fold((Some(OrderedFloat::max_value()), None), |(min, max), e| {
                (min.min(e), max.max(e))
            });
        let min = min.unwrap_or_default();
        let max = max.unwrap_or_default();

        // Normalize to [MIN, MAX]
        let scale = OrderedFloat((1u64 << (size - 1)) as _);
        let weights = if min < max {
            weights
                .into_iter()
                .map(|weight| {
                    weight
                        .map(|w| ((w - min) / (max - min) * scale).floor().0 as u64)
                        .unwrap_or(DEFAULT_WEIGHT)
                })
                .collect()
        } else {
            // uniform dist
            vec![DEFAULT_WEIGHT; weights.len()]
        };

        // Classify
        let mask = (1u64 << size) - 1;
        let size: usize = size as _;
        let filters: Vec<Vec<_>> = weights
            .into_iter()
            .map(|w| {
                let mut bits = (w & mask).max(1);
                if accumulate {
                    for offset in (0..size).rev() {
                        if (bits >> offset) & 1 == 1 {
                            bits = (1 << (offset + 1)) - 1;
                            break;
                        }
                    }
                }
                (0..size)
                    .map(move |offset| (bits >> offset) & 1 == 1)
                    .collect()
            })
            .collect();

        // Collect data
        let data = (0..size)
            .map(|offset| {
                data.iter()
                    .zip(filters.iter())
                    .filter(|&(_, filters)| filters[offset])
                    .map(|(item, _)| item.clone())
                    .collect()
            })
            .collect();

        Ok(Self { data })
    }
}

fn build_service_reference_url_by_raw(name: &str, namespace: &str, port: u16) -> Result<Url> {
    format!("http://{name}.{namespace}.svc:{port}")
        .parse()
        .map_err(|error| anyhow!("Failed to parse service URL: {error}"))
}

pub(crate) fn build_service_reference_url_by_class(class: &HistogramClassCrd) -> Result<Url> {
    let HistogramClassCrd {
        spec:
            HistogramClassSpec {
                backend_ref:
                    ServiceReference {
                        object:
                            ObjectReference {
                                name, namespace, ..
                            },
                        port,
                    },
                ..
            },
        ..
    } = class;

    let namespace = namespace.as_deref().ok_or_else(|| {
        anyhow!(
            "Required backend namespace: {kind}/???/{name}",
            kind = HistogramClassCrd::kind(&()),
        )
    })?;
    let port = *port;
    build_service_reference_url_by_raw(name, namespace, port)
}

pub(crate) fn build_service_reference_url_by_service(svc: &Service, port: u16) -> Result<Url> {
    let name = svc.name_any();
    let namespace = svc.namespace().expect("namespaced resource");
    build_service_reference_url_by_raw(&name, &namespace, port)
}
