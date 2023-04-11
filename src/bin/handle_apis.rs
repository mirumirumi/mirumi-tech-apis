use anyhow::{Error, Result};
use aws_sdk_dynamodb::types::AttributeValue;
use axum::{
    body::Body as AxumBody,
    error_handling::HandleError,
    extract::{Extension, Path, Query},
    http::{Request as AxumRequest, StatusCode},
    response::{IntoResponse, Json, Result as AxumResult},
    routing::{get, post},
    Router,
};
use http::{
    header::{self, HeaderName},
    Request as HttpRequest, Response as HttpResponse,
};
use lambda_http::{
    http::Method,
    request::RequestContext,
    run, service_fn,
    tower::{layer::layer_fn, Layer, Service, ServiceBuilder, ServiceExt},
    Body, Error as LambdaError, Request as LambdaRequest, RequestExt, Response,
};
use once_cell::sync::Lazy;
use serde::{
    ser::{SerializeMap, SerializeSeq, Serializer},
    Deserialize, Serialize,
};
use serde_json::{json, Value};
use std::{cmp::Ordering, collections::HashMap, convert::Infallible, env, str::FromStr, sync::Arc};
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

mod utils {
    pub mod dynamodb;
    pub mod lambda;
    pub mod responses;
}

use utils::{dynamodb, lambda, responses::*};

#[rustfmt::skip]
static ENV_NAME: Lazy<String> = Lazy::new(|| env::var("ENV_NAME").expect("\"ENV_NAME\" env var is not set."));
#[rustfmt::skip]
static POST_TABLE_NAME: Lazy<String> = Lazy::new(|| env::var("POST_TABLE_NAME").expect("\"POST_TABLE_NAME\" env var is not set."));
#[rustfmt::skip]
static PAGE_ITEMS: Lazy<usize> = Lazy::new(|| 13);
// #[rustfmt::skip]
// static DB_ENDPOINT: Lazy<&str> = Lazy::new(|| "postgresql://main:rL0FlUoHOzm4LGCEdsJcpA@mirumi-tech-4368.6xw.cockroachlabs.cloud:26257/mirumi-tech-4368.{os.environ['ENV_NAME']}?sslmode=verify-full&sslrootcert=./root.crt");

#[derive(Clone)]
struct Sdk {
    dynamodb: aws_sdk_dynamodb::Client,
}

#[tokio::main]
async fn main() -> Result<(), LambdaError> {
    // let log_service = service_fn(|request: LambdaRequest| async move {
    //     let context = request.lambda_context();
    //     lambda::log_incoming_event(&request, context);
    //     Ok::<_, Infallible>(http::Response::new(()))
    // });

    let config = aws_config::load_from_env().await;
    let dynamodb = aws_sdk_dynamodb::Client::new(&config);

    let sdk = Sdk { dynamodb };

    let router = Router::new()
        .nest(
            format!("/mirumitech-{}-apis", &*ENV_NAME).as_str(),
            Router::new()
                .route("/get-top-indexes", get(get_top_indexes))
                .route("/get-post", get(get_post)),
        )
        .layer(
            ServiceBuilder::new()
                .layer(Extension(Arc::new(sdk)))
                .layer(lambda::init_app()),
        );

    // let app = LogLayer::new(log_service).layer(router);

    run(router).await
}

async fn get_top_indexes(
    Extension(sdk): Extension<Arc<Sdk>>,
    Query(query_params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let page = query_params.get("page").unwrap();

    let res = sdk
        .dynamodb
        .scan()
        .table_name(&*POST_TABLE_NAME)
        .projection_expression("slag, title, created_at, updated_at")
        .send()
        .await
        .unwrap();

    let mut items: Vec<HashMap<String, AttributeValue>> = res.items().unwrap().to_vec();

    let count = items.len();

    items.sort_by(|a, b| {
        let a_created_at = a.get("created_at").unwrap().as_s().unwrap();
        let b_created_at = b.get("created_at").unwrap().as_s().unwrap();
        a_created_at.cmp(b_created_at).reverse()
    });

    let mut result: Vec<HashMap<String, AttributeValue>> = items;

    if page == "all" {
        // In "all-entries" page
    } else {
        // In top indexes (contains "page/1")

        let page = page
            .parse::<usize>()
            .expect("Failed to parse page number to i32 from String.");

        let start = (page - 1) * (&*PAGE_ITEMS);
        let end = (page) * (&*PAGE_ITEMS);
        result = result[start..end].to_vec();
    }

    Json(json!({
        "items": result
                .iter()
                .map(|item| serde_json::to_value(dynamodb::AttributeValueMap(item.clone())).unwrap())
                .collect::<Vec<serde_json::Value>>(),
        "counte": count,
    }))
}

async fn get_post(
    Extension(sdk): Extension<Arc<Sdk>>,
    Query(query_params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let slag = query_params.get("slag").unwrap();

    let res = sdk
        .dynamodb
        .get_item()
        .table_name(&*POST_TABLE_NAME)
        .key("slag", AttributeValue::S(slag.to_string()))
        .send()
        .await
        .unwrap();

    let item = res.item().unwrap();

    Json(json!(serde_json::to_value(dynamodb::AttributeValueMap(
        item.to_owned()
    ))
    .unwrap()))
}

// struct LogLayer<S> {
//     log_service: S,
// }

// impl<S> LogLayer<S> {
//     fn new(log_service: S) -> Self {
//         Self { log_service }
//     }
// }

// impl<S, Inner> Layer<Inner> for LogLayer<S>
// where
//     S: Service<LambdaRequest, Response = http::Response<()>, Error = Infallible> + Clone,
//     Inner: Service<HttpRequest<Body>>,
// {
//     type Service = LogService<S, Inner, Inner::Response, Inner::Error>;

//     fn layer(&self, inner: Inner) -> Self::Service {
//         LogService {
//             log_service: self.log_service.clone(),
//             inner,
//             _marker: PhantomData,
//         }
//     }
// }

// use futures::{future::BoxFuture, Future};
// use std::marker::PhantomData;
// use std::pin::Pin;
// use std::task::{Context, Poll};

// pub struct LogService<S, Inner> {
//     log_service: S,
//     inner: Inner,
//     _phantom: PhantomData<fn() -> (S::Request, Inner::Request)>,
// }

// impl<S, Inner> Service<HttpRequest<Body>> for LogService<S, Inner>
// where
//     S: Service<HttpRequest<Body>, Response = (), Error = Infallible> + Clone,
//     Inner: Service<HttpRequest<Body>> + Clone,
// {
//     type Response = Inner::Response;
//     type Error = Inner::Error;
//     type Future = Inner::Future;

//     fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
//         self.inner.poll_ready(cx)
//     }

//     fn call(&mut self, request: HttpRequest<Body>) -> Self::Future {
//         let context = request.lambda_context();
//         lambda::log_incoming_event(&request, context);

//         self.inner.call(request)
//     }
// }
