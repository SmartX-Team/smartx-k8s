use polars::{error::PolarsResult, prelude::*};

pub(crate) struct Lifecycle {
    pub(crate) pre_start: bool,
    pub(crate) post_stop: bool,
}

pub(crate) struct Resource {
    pub(crate) priority: i32,
    pub(crate) min: Option<f64>,
    pub(crate) max: Option<f64>,
    pub(crate) weight: u64,
}

pub(crate) struct Item<'a, T> {
    pub(crate) claim_name: &'a str,
    pub(crate) lifecycle: Lifecycle,
    pub(crate) resource: Resource,
    pub(crate) item: &'a T,
}

pub(crate) struct ScheduledItem<'a, S, T> {
    pub(crate) item: &'a T,
    pub(crate) resources: Vec<S>,
}

pub(crate) fn schedule<'a, S, T>(
    items: Vec<Item<'a, T>>,
    resources: super::WeightedItems<S>,
) -> PolarsResult<Vec<ScheduledItem<'a, S, T>>>
where
    T: 'a,
{
    // FIXME: 우선순위와 가중치 (동일우선순위 내) 따라 공정분배하기 (스케줄링!)
    // FIXME: BestEffort (HIGH) 용 metric 과 Guaranteed (LOW) 용 metric을 분리!
    // FIXME:   - HIGH: 지킬 필요는 없지만 자원이 남아돈다면 존중할 수 있는 weight
    // FIXME:   - LOW: 우선순위별 반드시 지켜야할 weight
    // FIXME:   - 메트릭은 pool 에서만 정의할 수 있고, claim 에서는 정의할 수 없다 (claim 의 스푸핑 공격 방어)
    // FIXME:   - Polars 등 고급 도구를 이용할까?
    // FIXME:   - 로직 작동 순서를 **엄밀하게** 정의해야 함

    const COL_CLAIM_NAME: PlSmallStr = PlSmallStr::from_static("claim");
    const COL_ITEM: PlSmallStr = PlSmallStr::from_static("item");
    const COL_LIFECYCLE_PRE_START: PlSmallStr = PlSmallStr::from_static("pre_start");
    const COL_LIFECYCLE_POST_STOP: PlSmallStr = PlSmallStr::from_static("post_stop");
    const COL_PRIORITY: PlSmallStr = PlSmallStr::from_static("priority");
    const COL_WEIGHT: PlSmallStr = PlSmallStr::from_static("weight");
    const COL_WEIGHT_MAX: PlSmallStr = PlSmallStr::from_static("max_weight");
    const COL_WEIGHT_MIN: PlSmallStr = PlSmallStr::from_static("min_weight");

    let super::WeightedItems {
        items: resources,
        weights,
    } = resources;

    // Collect items into a dataframe
    let df_items = {
        // Collect metadata
        let claim_name = Column::new(
            COL_CLAIM_NAME,
            items.iter().map(|item| item.claim_name).collect::<Series>(),
        );

        // Collect lifecycle
        let lifecycle_pre_start = Column::new(
            COL_LIFECYCLE_PRE_START,
            items
                .iter()
                .map(|item| item.lifecycle.pre_start)
                .collect::<Series>(),
        );
        let lifecycle_post_stop = Column::new(
            COL_LIFECYCLE_POST_STOP,
            items
                .iter()
                .map(|item| item.lifecycle.post_stop)
                .collect::<Series>(),
        );

        // Collect resource
        let priority = Column::new(
            COL_PRIORITY,
            items
                .iter()
                .map(|item| item.resource.priority)
                .collect::<Series>(),
        );
        let min = Column::new(
            COL_WEIGHT_MIN,
            items
                .iter()
                .map(|item| item.resource.min)
                .collect::<Series>(),
        );
        let max = Column::new(
            COL_WEIGHT_MAX,
            items
                .iter()
                .map(|item| item.resource.max)
                .collect::<Series>(),
        );
        let weight = Column::new(
            COL_WEIGHT,
            items
                .iter()
                .map(|item| item.resource.weight)
                .collect::<Series>(),
        );

        DataFrame::new(vec![
            claim_name,
            lifecycle_pre_start,
            lifecycle_post_stop,
            priority,
            min,
            max,
            weight,
        ])?
    };

    // Collect resources into a dataframe
    let df_resources = {
        let weight = Column::new(
            COL_WEIGHT,
            weights
                .iter()
                .copied()
                .map(|item| item.map(|x| x.0))
                .collect::<Series>(),
        );

        DataFrame::new(vec![weight])?
    };

    dbg!(df_items);
    dbg!(df_resources);
    todo!()
}
