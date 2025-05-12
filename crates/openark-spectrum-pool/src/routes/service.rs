use std::collections::BTreeMap;

use actix_web::{HttpResponse, Responder, post, web};
use anyhow::Result;
use openark_spectrum_api::{
    common::ObjectReference,
    pool_claim::PoolResourceLifecycle,
    schema::{CommitState, PoolCommitRequest, PoolRequest, PoolResponse},
};
#[cfg(feature = "tracing")]
use tracing::{Level, instrument, warn};

use crate::store::Store;

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO, skip_all))]
#[post("")]
async fn post(
    store: web::Data<Store>,
    web::Json(args): web::Json<PoolRequest<'static>>,
) -> impl Responder {
    match try_handle(store, args) {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(error) => {
            #[cfg(feature = "tracing")]
            warn!("failed to collect service binding states: {error}");
            HttpResponse::Forbidden().json("Err")
        }
    }
}

fn try_handle(store: web::Data<Store>, args: PoolRequest<'static>) -> Result<PoolResponse> {
    let PoolRequest {
        resources,
        namespace,
    } = args;

    store
        .read(|txn| {
            resources
                .into_iter()
                .map(|name| {
                    txn.get(&ObjectReference {
                        group: "discovery.k8s.io".into(),
                        kind: "Endpoint".into(),
                        name: name.into_owned(),
                        namespace: Some(namespace.clone()),
                    })
                })
                .collect::<Result<_, _>>()
        })
        .map(|binded| PoolResponse { binded })
        .map_err(Into::into)
}

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO, skip_all))]
#[post("commit")]
async fn post_commit(
    store: web::Data<Store>,
    web::Json(args): web::Json<PoolCommitRequest<'static>>,
) -> impl Responder {
    match try_handle_commit(store, args) {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(error) => {
            #[cfg(feature = "tracing")]
            warn!("failed to commit service binding states: {error}");
            HttpResponse::Forbidden().json("Err")
        }
    }
}

fn try_handle_commit<'a>(store: web::Data<Store>, args: PoolCommitRequest<'a>) -> Result<()> {
    let PoolCommitRequest { items } = args;

    // Collect last states
    let last_states = store.read(|txn| {
        items
            .iter()
            .map(|item| {
                item.pool
                    .resources
                    .iter()
                    .map(|name| {
                        txn.get(&ObjectReference {
                            group: "discovery.k8s.io".into(),
                            kind: "Endpoint".into(),
                            name: name.to_string(),
                            namespace: Some(item.pool.namespace.clone()),
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()
            })
            .collect::<Result<Vec<_>, _>>()
    })?;

    #[derive(Debug)]
    struct Value<'a> {
        address: &'a str,
        claim_name: &'a str,
        lifecycle: &'a PoolResourceLifecycle,
        resource: ObjectReference,
    }

    // Order resources by (priority, order, claim, name)
    let orders: BTreeMap<_, _> = items
        .iter()
        .zip(last_states)
        .enumerate()
        .flat_map(|(item_index, (item, last_claims))| {
            let claim_name = item.name.as_ref();
            let priority = item.priority;

            item.pool
                .resources
                .iter()
                .zip(last_claims)
                .filter(|(_, last)| last.claim.as_deref() != Some(claim_name))
                .enumerate()
                .map(move |(resource_index, (name, _))| {
                    let address = name.as_ref(); // endpoint.addresses[0]
                    let name = name.as_ref();
                    let order = (resource_index, item_index);
                    let key = (priority, order, claim_name, name);
                    let value = Value {
                        address,
                        claim_name,
                        lifecycle: &item.lifecycle,
                        resource: ObjectReference {
                            group: "discovery.k8s.io".into(),
                            kind: "Endpoint".into(),
                            name: name.to_string(),
                            namespace: Some(item.pool.namespace.clone()),
                        },
                    };
                    (key, value)
                })
        })
        .collect();

    store.write(|txn| {
        for Value {
            address,
            claim_name,
            lifecycle,
            resource,
        } in orders.values()
        {
            match txn.put(resource, *claim_name, *address, *lifecycle)? {
                CommitState::Pending => break,
                CommitState::Preparing | CommitState::Running => continue,
            }
        }
        Ok(())
    })
}
