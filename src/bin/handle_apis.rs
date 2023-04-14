#[allow(unused_imports)]
use anyhow::{Error, Result};
#[allow(unused_imports)]
use aws_sdk_dynamodb::types::{AttributeValue, Condition, ExpectedAttributeValue};
#[allow(unused_imports)]
use axum::{
    body::Body as AxumBody,
    error_handling::HandleError,
    extract::{Extension, Path, Query},
    http::{Request as AxumRequest, StatusCode},
    response::{IntoResponse, Json, Result as AxumResult},
    routing::{get, post},
    Router,
};
#[allow(unused_imports)]
use http::{
    header::{self, HeaderName},
    Request as HttpRequest, Response as HttpResponse,
};
#[allow(unused_imports)]
use lambda_http::{
    http::Method,
    request::RequestContext,
    run, service_fn,
    tower::{layer::layer_fn, Layer, Service, ServiceBuilder, ServiceExt},
    Body, Error as LambdaError, Request as LambdaRequest, RequestExt, Response,
};
use once_cell::sync::Lazy;
use percent_encoding::{utf8_percent_encode, CONTROLS};
#[allow(unused_imports)]
use serde::{
    ser::{SerializeMap, SerializeSeq, Serializer},
    Deserialize, Serialize,
};
use serde_json::json;
#[allow(unused_imports)]
use std::{cmp::Ordering, collections::HashMap, convert::Infallible, env, str::FromStr, sync::Arc};
#[allow(unused_imports)]
use tower_http::cors::{Any, CorsLayer};
#[allow(unused_imports)]
use tracing::info;

mod utils {
    pub mod dynamodb;
    pub mod lambda;
    pub mod responses;
}

use utils::{
    dynamodb::{AttributeValueItem, ListToVec},
    lambda,
};

#[rustfmt::skip]
static ENV_NAME: Lazy<String> = Lazy::new(|| env::var("ENV_NAME").expect("\"ENV_NAME\" env var is not set."));
#[rustfmt::skip]
static POST_TABLE_NAME: Lazy<String> = Lazy::new(|| env::var("POST_TABLE_NAME").expect("\"POST_TABLE_NAME\" env var is not set."));
#[rustfmt::skip]
static PAGE_ITEMS: Lazy<usize> = Lazy::new(|| 13);

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
                .route("/get-post", get(get_post))
                .route("/get-all-tags", get(get_all_tags))
                .route("/get-tag-indexes", get(get_tag_indexes))
                .route("/search-post", get(search_post)),
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
        // 記事一覧および全記事一覧にリンクが存在していなければ generate もされないので get-post で単独の対応は不要
        .filter_expression("attribute_not_exists(publish) OR publish = :publish")
        .expression_attribute_values(":publish", AttributeValue::Bool(true))
        .projection_expression("slag, title, created_at, updated_at")
        .send()
        .await
        .unwrap();

    let mut items: Vec<HashMap<String, AttributeValue>> = res.items().unwrap().to_vec();

    let count = items.len();

    sort_by_created_at(&mut items);

    let mut result: Vec<HashMap<String, AttributeValue>> = items;

    if page != "all" {
        // In top indexes (contains "page/1") (do nothing when `page == "all`)

        let page = page
            .parse::<usize>()
            .expect("Failed to parse page number to usize from String.");

        result = slice_posts(result, page, count);
    }

    Json(json!({
        "items": result
                .iter()
                .map(|item| serde_json::to_value(AttributeValueItem(item.clone())).unwrap())
                .collect::<Vec<serde_json::Value>>(),
        "count": count,
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

    Json(json!(serde_json::to_value(AttributeValueItem(
        item.to_owned()
    ))
    .unwrap()))
}

#[derive(Clone)]
struct TableTagData {
    tags: Vec<String>,
    search_tags: Vec<String>,
}

#[derive(Serialize)]
struct SearchResult {
    tag: String,
    search_tag: String,
}

impl SearchResult {
    fn exists_tag(result: &[SearchResult], tag: &str) -> bool {
        result.iter().any(|result| result.tag == tag)
    }
}

async fn get_all_tags(Extension(sdk): Extension<Arc<Sdk>>) -> impl IntoResponse {
    let res = sdk
        .dynamodb
        .scan()
        .table_name(&*POST_TABLE_NAME)
        .filter_expression("attribute_not_exists(publish) OR publish = :publish")
        .expression_attribute_values(":publish", AttributeValue::Bool(true))
        .projection_expression("tags, search_tags")
        .send()
        .await
        .unwrap();

    let posts: Vec<TableTagData> = res
        .items()
        .unwrap()
        .into_iter()
        .map(|item| {
            let tags = AttributeValueItem(item.clone()).list_to_vec("tags");
            let search_tags = AttributeValueItem(item.clone()).list_to_vec("search_tags");
            TableTagData { tags, search_tags }
        })
        .collect();

    let mut result: Vec<SearchResult> = vec![];

    for post in posts {
        for (i, tag) in post.tags.iter().enumerate() {
            if !SearchResult::exists_tag(&result, &tag) {
                result.push(SearchResult {
                    tag: tag.to_string(),
                    search_tag: post.search_tags[i].clone(),
                })
            }
        }
    }

    result.sort_unstable_by(|a, b| a.tag.cmp(&b.tag));

    Json(json!(result))
}

async fn get_tag_indexes(
    Extension(sdk): Extension<Arc<Sdk>>,
    Query(query_params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let page = query_params.get("page").unwrap();
    let tag = query_params.get("tag").unwrap(); // No encoded (link string, not title)

    let encoded_tag = utf8_percent_encode(tag, CONTROLS).to_string();

    let res = sdk
        .dynamodb
        .scan()
        .table_name(&*POST_TABLE_NAME)
        .filter_expression("contains(search_tags, :encoded_tag) AND (attribute_not_exists(publish) OR publish = :publish)")
        .expression_attribute_values(":encoded_tag", AttributeValue::S(encoded_tag))
        .expression_attribute_values(":publish", AttributeValue::Bool(true))
        .projection_expression("slag, title, created_at, updated_at")
        .send()
        .await
        .unwrap();

    let mut items = res.items().unwrap().to_vec();
    let count = items.len();

    sort_by_created_at(&mut items);

    let mut result: Vec<HashMap<String, AttributeValue>> = items;

    let page = page
        .parse::<usize>()
        .expect("Failed to parse page number to usize from String.");

    result = slice_posts(result, page, count);

    Json(json!({
        "items": result
                .iter()
                .map(|item| serde_json::to_value(AttributeValueItem(item.clone())).unwrap())
                .collect::<Vec<serde_json::Value>>(),
        "count": count,
    }))
}

async fn search_post(
    Extension(sdk): Extension<Arc<Sdk>>,
    Query(query_params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let query = query_params.get("query").unwrap().to_lowercase();

    let queries: Vec<&str> = query.split_whitespace().collect();
    let mut candidates: Vec<_> = vec![];

    for (i, q) in queries.iter().enumerate() {
        let res = sdk
            .dynamodb
            .scan()
            .table_name(&*POST_TABLE_NAME)
            .filter_expression("(contains(slag, :query) OR contains(search_title, :q) OR contains(search_tags, :q) OR contains(search_tags, :joined)) AND (attribute_not_exists(publish) OR publish = :publish)")
            .expression_attribute_values(":query", AttributeValue::S(query.clone()))
            .expression_attribute_values(":q", AttributeValue::S(q.to_string()))
            .expression_attribute_values(":joined", AttributeValue::S(queries.join("-")))
            .expression_attribute_values(":publish", AttributeValue::Bool(true))
            .projection_expression("slag, title, created_at, updated_at")
            .send()
            .await
            .unwrap();

        let items = res.items().unwrap().to_vec();

        if i == 0 {
            candidates = items;
        } else {
            for item in items {
                candidates = candidates
                    .into_iter()
                    .filter(|candidate| candidate.get("slag").unwrap() != item.get("slag").unwrap())
                    .collect();
            }
        }
    }

    Json(json!(candidates
        .iter()
        .map(|item| serde_json::to_value(AttributeValueItem(item.clone())).unwrap())
        .collect::<Vec<serde_json::Value>>()))
}

fn sort_by_created_at(items: &mut Vec<HashMap<String, AttributeValue>>) {
    items.sort_unstable_by(|a, b| {
        let a_created_at = a.get("created_at").unwrap().as_s().unwrap();
        let b_created_at = b.get("created_at").unwrap().as_s().unwrap();
        a_created_at.cmp(b_created_at).reverse()
    });
}

fn slice_posts<T: std::clone::Clone>(posts: Vec<T>, page: usize, count: usize) -> Vec<T> {
    let collect_page_num = count / &*PAGE_ITEMS;
    let remainder_page_num = count % &*PAGE_ITEMS;

    if page <= collect_page_num {
        let start = (page - 1) * (&*PAGE_ITEMS);
        let end = (page) * (&*PAGE_ITEMS);
        posts[start..end].to_vec()
    } else {
        posts[..remainder_page_num].to_vec()
    }
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
