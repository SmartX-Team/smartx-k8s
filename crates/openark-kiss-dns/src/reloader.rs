use std::{net::IpAddr, str::FromStr, sync::Arc, time::Duration};

use anyhow::Result;
use futures::{TryStreamExt, stream::FuturesUnordered};
use hickory_server::{
    authority::ZoneType,
    proto::rr::{
        Name, RData, Record,
        rdata::{A, AAAA},
    },
    store::in_memory::InMemoryAuthority,
};
use kube::{
    Api, Client, ResourceExt,
    runtime::watcher::{Config, Error, Event, watcher},
};
use openark_kiss_api::r#box::BoxCrd;
use tokio::time::sleep;
use tracing::{Level, error, info, instrument, warn};

pub(super) struct ReloaderContext {
    origins: Vec<Name>,
}

impl ReloaderContext {
    pub(super) async fn try_new(domain: &str) -> Result<Self> {
        Ok(Self {
            origins: vec![
                Name::from_str(".")?,
                Name::from_str("box")?,
                Name::from_str("node")?,
                Name::from_str(domain)?,
                Name::from_str(&format!("box.{domain}"))?,
                Name::from_str(&format!("node.{domain}"))?,
            ],
        })
    }
}

pub(super) async fn loop_forever(ctx: ReloaderContext, kube: Client, handler: super::Handler) {
    let api = Api::all(kube);

    loop {
        let handle_event = |e| handle_event(&ctx, &handler, e);
        if let Err(error) = watcher(api.clone(), Config::default())
            .try_for_each(handle_event)
            .await
        {
            error!("failed to operate reloader: {error}");

            let interval = Duration::from_secs(5);
            warn!("restarting reloader in {interval:?}...");
            sleep(interval).await
        }
    }
}

#[instrument(level = Level::INFO, skip(ctx, handler, event))]
async fn handle_event(
    ctx: &ReloaderContext,
    handler: &super::Handler,
    event: Event<BoxCrd>,
) -> Result<(), Error> {
    match event {
        Event::Apply(object) | Event::InitApply(object) => handle_apply(ctx, handler, object).await,
        Event::Delete(object) => handle_delete(ctx, handler, object).await,
        Event::Init | Event::InitDone => Ok(()),
    }
}

#[instrument(level = Level::INFO, skip(ctx, handler, object))]
async fn handle_apply(
    ctx: &ReloaderContext,
    handler: &super::Handler,
    object: BoxCrd,
) -> Result<(), Error> {
    let ReloaderContext { origins } = ctx;

    let addr = match object
        .status
        .as_ref()
        .and_then(|status| status.access.primary)
        .map(|spec| spec.address)
    {
        Some(addr) => addr,
        None => return handle_delete(ctx, handler, object).await,
    };

    let name = object.name_any();
    let name = name.as_str();
    info!("Applying box: {name}");

    origins
        .iter()
        .map(|origin| async move {
            let name = Name::from_str(name)
                .and_then(|name| name.append_domain(origin))
                .map_err(handle_error)?;

            let zone_type = ZoneType::Primary;
            let allow_axfr = false;
            let nx_proof_kind = None;
            let authority =
                InMemoryAuthority::empty(origin.clone(), zone_type, allow_axfr, nx_proof_kind);

            let ttl = 300;
            let rdata = match addr {
                IpAddr::V4(addr) => RData::A(A(addr)),
                IpAddr::V6(addr) => RData::AAAA(AAAA(addr)),
            };
            let record = Record::from_rdata(name.clone(), ttl, rdata);

            let serial = 0;
            authority.upsert(record, serial).await;

            handler
                .catalog
                .write()
                .await
                .upsert(name.into(), vec![Arc::new(authority)]);
            Ok(())
        })
        .collect::<FuturesUnordered<_>>()
        .try_collect()
        .await
}

#[instrument(level = Level::INFO, skip(ctx, handler, object))]
async fn handle_delete(
    ctx: &ReloaderContext,
    handler: &super::Handler,
    object: BoxCrd,
) -> Result<(), Error> {
    let ReloaderContext { origins } = ctx;

    let name = object.name_any();
    let name = name.as_str();
    info!("Deleting box: {name}");

    origins
        .iter()
        .map(|origin| async move {
            let name = Name::from_str(name)
                .and_then(|name| name.append_domain(origin))
                .map_err(handle_error)?;

            handler.catalog.write().await.remove(&name.into());
            Ok(())
        })
        .collect::<FuturesUnordered<_>>()
        .try_collect()
        .await
}

fn handle_error(error: impl Into<Box<dyn ::std::error::Error + Send + Sync>>) -> Error {
    Error::WatchFailed(::kube::Error::Service(error.into()))
}
