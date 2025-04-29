use std::collections::BTreeMap;

use gateway_api::apis::experimental::httproutes::HTTPRouteSpec;
#[cfg(feature = "kube")]
use kube::CustomResource;
#[cfg(feature = "schemars")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{traffic_route_claim::RouteClaimStatus, traffic_router_class::PartialObjectReference};

#[derive(Clone, Debug)]
#[cfg_attr(feature = "kube", derive(CustomResource))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "kube",
    kube(
        namespaced,
        category = "org",
        group = "org.ulagbulag.io",
        version = "v1alpha1",
        kind = "HTTPRouteClaim",
        root = "HTTPRouteClaimCrd",
        status = "RouteClaimStatus",
        printcolumn = r#"{
            "name": "class",
            "type": "string",
            "jsonPath": ".spec.trafficRouterClassName"
        }"#,
        printcolumn = r#"{
            "name": "accepted",
            "type": "string",
            "jsonPath": ".status.conditions[?(@.type==\"Accepted\")].status"
        }"#,
        printcolumn = r#"{
            "name": "age",
            "type": "date",
            "jsonPath": ".metadata.creationTimestamp"
        }"#,
        printcolumn = r#"{
            "name": "version",
            "type": "integer",
            "priority": 1,
            "description": "claim version",
            "jsonPath": ".metadata.generation"
        }"#
    )
)]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct HTTPRouteClaimSpec {
    /// trafficRouterClassName is the name of the class that is managing
    /// Routers of this class.
    pub traffic_router_class_name: String,

    /// resources is the requested resources to provision.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub resources: Vec<RouteResource>,

    /// template is the HTTPRoute template to build.
    #[serde(default)]
    pub template: HTTPRouteSpec,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]

pub struct RouteResource {
    pub backend_ref: RouteResourceBackendRef,

    #[cfg_attr(feature = "serde", serde(default))]
    pub lifecycle: RouteResourceLifecycle,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]

pub struct RouteResourceBackendRef {
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub object: PartialObjectReference,

    pub port: u16,
}

#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]

pub struct RouteResourceLifecycle {
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub pre_start: Option<RouteResourceProbe>,

    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub post_stop: Option<RouteResourceProbe>,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]

pub enum RouteResourceProbe {
    Http(RouteResourceHTTPProbe),
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]

pub struct RouteResourceHTTPProbe {
    pub path: String,

    pub port: u16,

    pub protocol: RouteResourceHTTPProtocol,

    #[cfg_attr(feature = "serde", serde(flatten))]
    pub body: RouteResourceHTTPBody,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]

pub enum RouteResourceHTTPProtocol {
    DELETE,
    GET,
    PATCH,
    POST,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]

pub enum RouteResourceHTTPBody {
    JsonBody(BTreeMap<String, Value>),
}
