use openark_spectrum_api::histogram::HistogramSettings;
use ordered_float::{Float, OrderedFloat};

use crate::status::{Reason, Status};

const DEFAULT_WEIGHT: u64 = (u64::MIN + u64::MAX) / 2;

pub(crate) struct Histogram<T> {
    pub(crate) data: Vec<Vec<T>>,
}

impl<T> Histogram<T> {
    pub(crate) fn build(
        settings: &HistogramSettings,
        data: &[T],
        weights: Vec<Option<OrderedFloat<f64>>>,
    ) -> Result<Self, Status>
    where
        T: Clone,
    {
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
            return Err(Status {
                reason: Reason::InvalidHistogram,
                message: "Invalid histogram size: Too small".into(),
                requeue: false,
            });
        } else if size as u32 > u64::BITS - 1 {
            return Err(Status {
                reason: Reason::InvalidHistogram,
                message: "Invalid histogram size: Too large".into(),
                requeue: false,
            });
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
