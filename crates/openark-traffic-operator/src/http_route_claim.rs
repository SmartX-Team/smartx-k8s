use anyhow::Result;
use gateway_api::apis::experimental::httproutes::{HTTPRoute, HTTPRouteSpec};
use kube::{Client, api::ObjectMeta};
use openark_traffic_api::http_route_claim::{HTTPRouteClaimCrd, RouteResource};

use crate::traffic_route_claim::TrafficRouteClaim;

impl TrafficRouteClaim for HTTPRouteClaimCrd {
    type Target = HTTPRoute;
    type TargetSpec = HTTPRouteSpec;

    #[inline]
    fn build_target(
        metadata: ObjectMeta,
        spec: <Self as TrafficRouteClaim>::TargetSpec,
    ) -> <Self as TrafficRouteClaim>::Target {
        HTTPRoute {
            metadata,
            spec,
            status: None,
        }
    }

    #[inline]
    fn metadata(&self) -> &ObjectMeta {
        &self.metadata
    }

    #[inline]
    fn class_name(&self) -> &str {
        &self.spec.traffic_router_class_name
    }

    #[inline]
    fn resources(&self) -> &[RouteResource] {
        &self.spec.resources
    }

    #[inline]
    fn template(&self) -> &<Self as TrafficRouteClaim>::TargetSpec {
        &self.spec.template
    }
}

#[inline]
pub async fn loop_forever(args: super::Args, client: Client) -> Result<()> {
    crate::traffic_route_claim::loop_forever::<HTTPRouteClaimCrd>(args, client).await
}
