use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use actix_web::rt::spawn;
use anyhow::Result;
use openark_spectrum_api::{
    pool_claim::{PoolResourceHttpBody, PoolResourceHttpProbe, PoolResourceProbe},
    schema::CommitState,
};
use reqwest::{Client, Error};
#[cfg(feature = "tracing")]
use tracing::{error, info};

async fn execute_probe_http(
    client: &Client,
    address: &str,
    probe: &PoolResourceHttpProbe,
) -> Result<()> {
    let PoolResourceHttpProbe {
        method,
        path,
        port,
        secure,
        body,
    } = probe;

    let protocol = if *secure { "https" } else { "http" };
    let method = (*method).into();
    let url = format!("{protocol}://{address}:{port}{path}");
    let builder = client.request(method, &url);

    #[cfg(feature = "tracing")]
    match body {
        Some(PoolResourceHttpBody::JsonBody(body)) => info!("Start probe: {url:?} {body:?}"),
        None => info!("Start probe: {url:?}"),
    }

    let builder = match body {
        Some(PoolResourceHttpBody::JsonBody(body)) => builder.json(body),
        None => builder,
    };

    let response = builder.send().await?;
    let mut response = response.error_for_status()?;

    // Consume chunks
    while let Some(_) = response.chunk().await? {}

    #[cfg(feature = "tracing")]
    {
        let status = response.status();
        info!("Complete probe: {url:?} {{{status}}}");
    }

    Ok(())
}

async fn execute_probe(client: &Client, address: &str, probe: &PoolResourceProbe) -> Result<()> {
    match probe {
        PoolResourceProbe::Http(probe) => execute_probe_http(client, address, probe).await,
    }
}

pub(crate) struct Pool {
    client: Client,
    semaphore: Arc<AtomicUsize>,
}

impl Pool {
    pub fn new(size: usize) -> Result<Self, Error> {
        Ok(Self {
            client: Client::builder().pool_max_idle_per_host(size).build()?,
            semaphore: Arc::new(AtomicUsize::new(size)),
        })
    }

    pub fn commit<F>(
        &self,
        address: &str,
        probes: &[PoolResourceProbe],
        on_completed: F,
    ) -> Result<CommitState>
    where
        F: 'static + FnOnce() -> Result<()>,
    {
        let semaphore = self.semaphore.clone();
        match semaphore.load(Ordering::SeqCst) {
            0 => Ok(CommitState::Pending),
            _ => {
                semaphore.fetch_sub(1, Ordering::SeqCst);
                let client = self.client.clone();
                let address = address.to_string();
                let probes = probes.to_vec();

                spawn(async move {
                    let mut result = Ok(());
                    for probe in &probes {
                        match execute_probe(&client, &address, &probe).await {
                            Ok(()) => continue,
                            Err(error) => {
                                result = Err(error);
                                break;
                            }
                        }
                    }
                    let result = result.and_then(|()| on_completed());

                    semaphore.fetch_add(1, Ordering::SeqCst);
                    match result {
                        Ok(()) => (),
                        Err(error) => {
                            #[cfg(feature = "tracing")]
                            {
                                error!("Failed to commit probe ({address}): {error}")
                            }

                            #[cfg(not(feature = "tracing"))]
                            {
                                let _ = error;
                            }
                        }
                    }
                });
                Ok(CommitState::Preparing)
            }
        }
    }
}
