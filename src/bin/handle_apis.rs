use anyhow::Result;
use aws_sdk_dynamodb::types::AttributeValue;
use lambda_http::{http::Method, run, Body, Error as LambdaError, Request, RequestExt, Response};
use once_cell::sync::Lazy;
use percent_encoding::{utf8_percent_encode, CONTROLS};
use serde::Serialize;
use serde_json::json;
use std::{collections::HashMap, env};
use tracing::error;

mod utils {
    pub mod dynamodb;
    pub mod lambda;
    pub mod responses;
}

use utils::{
    dynamodb::{AttributeValueItem, ListToVec},
    lambda,
    responses::*,
};

#[rustfmt::skip]
static POST_TABLE_NAME: Lazy<String> = Lazy::new(|| env::var("POST_TABLE_NAME").expect("\"POST_TABLE_NAME\" env var is not set."));
static PAGE_ITEMS: Lazy<usize> = Lazy::new(|| 13);

#[derive(Clone)]
struct Sdk {
    dynamodb: aws_sdk_dynamodb::Client,
}

#[tokio::main]
async fn main() -> Result<(), LambdaError> {
    let config = aws_config::load_from_env().await;
    let dynamodb = aws_sdk_dynamodb::Client::new(&config);

    let sdk = Sdk { dynamodb };

    run(lambda::init_app(|request| {
        lambda_handler(request, sdk.clone())
    }))
    .await
}

async fn lambda_handler(request: Request, sdk: Sdk) -> Result<Response<Body>, LambdaError> {
    let context = request.lambda_context();
    lambda::log_incoming_event(&request, context);

    let fullpath = request.uri().path();
    let path = &fullpath[fullpath
        .rfind("/")
        .expect("The requested URL does not include `/`.")..];
    let method = request.method();


    let result = match path {
        "/get-top-indexes" => match method {
            &Method::GET => get_top_indexes(&request, sdk).await,
            _ => _404(format!("No method found for `{}` path.", path)),
        },
        "/get-post" => match method {
            &Method::GET => get_post(&request, sdk).await,
            _ => _404(format!("No method found for `{}` path.", path)),
        },
        "/get-all-tags" => match method {
            &Method::GET => get_all_tags(&request, sdk).await,
            _ => _404(format!("No method found for `{}` path.", path)),
        },
        "/get-tag-indexes" => match method {
            &Method::GET => get_tag_indexes(&request, sdk).await,
            _ => _404(format!("No method found for `{}` path.", path)),
        },
        "/search-post" => match method {
            &Method::GET => search_post(&request, sdk).await,
            _ => _404(format!("No method found for `{}` path.", path)),
        },
        _ => _404("No API endpoint path found."),
    };

    match result {
        Ok(_) => result,
        Err(err) => {
            error!(err);
            // _500()
            panic!("")
        }
    }
}

async fn get_top_indexes(request: &Request, sdk: Sdk) -> Result<Response<Body>, LambdaError> {
    let query_params = request.query_string_parameters();
    let page = match query_params.first("page") {
        Some(page) => page,
        None => return _400("`page` query param is not found."),
    };

    let res = sdk
        .dynamodb
        .scan()
        .table_name(&*POST_TABLE_NAME)
        // If the link does not exist in the `/` and the `/all-entries`,
        // it will not be generated, so there is no need to use get-post alone.
        .filter_expression("attribute_not_exists(publish) OR publish = :publish")
        .expression_attribute_values(":publish", AttributeValue::Bool(true))
        .projection_expression("slag, title, created_at, updated_at")
        .send()
        .await?;

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

    let result = json!({
        "items": result
                .iter()
                .map(|item| serde_json::to_value(AttributeValueItem(item.clone())).unwrap())
                .collect::<Vec<serde_json::Value>>(),
        "count": count,
    })
    .to_string();
    _200(result)
}

async fn get_post(request: &Request, sdk: Sdk) -> Result<Response<Body>, LambdaError> {
    let query_params = request.query_string_parameters();
    let slag = match query_params.first("slag") {
        Some(slag) => slag,
        None => return _400("`slag` query param is not found."),
    };

    let res = sdk
        .dynamodb
        .get_item()
        .table_name(&*POST_TABLE_NAME)
        .key("slag", AttributeValue::S(slag.to_string()))
        .send()
        .await?;

    let item = res.item().unwrap();

    let result =
        json!(serde_json::to_value(AttributeValueItem(item.to_owned())).unwrap()).to_string();
    _200(result)
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

async fn get_all_tags(_request: &Request, sdk: Sdk) -> Result<Response<Body>, LambdaError> {
    let res = sdk
        .dynamodb
        .scan()
        .table_name(&*POST_TABLE_NAME)
        .filter_expression("attribute_not_exists(publish) OR publish = :publish")
        .expression_attribute_values(":publish", AttributeValue::Bool(true))
        .projection_expression("tags, search_tags")
        .send()
        .await?;

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

    let result = json!(result).to_string();
    _200(result)
}

async fn get_tag_indexes(request: &Request, sdk: Sdk) -> Result<Response<Body>, LambdaError> {
    let query_params = request.query_string_parameters();
    let page = match query_params.first("page") {
        Some(page) => page,
        None => return _400("`page` query param is not found."),
    };
    let tag = match query_params.first("tag") {
        // No encoded (link string, not title)
        Some(tag) => tag,
        None => return _400("`tag` query param is not found."),
    };

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
        .await?;

    let mut items = res.items().unwrap().to_vec();
    let count = items.len();

    sort_by_created_at(&mut items);

    let mut result: Vec<HashMap<String, AttributeValue>> = items;

    let page = page
        .parse::<usize>()
        .expect("Failed to parse page number to usize from String.");

    result = slice_posts(result, page, count);

    let result = json!({
        "items": result
                .iter()
                .map(|item| serde_json::to_value(AttributeValueItem(item.clone())).unwrap())
                .collect::<Vec<serde_json::Value>>(),
        "count": count,
    })
    .to_string();
    _200(result)
}

async fn search_post(request: &Request, sdk: Sdk) -> Result<Response<Body>, LambdaError> {
    let query_params = request.query_string_parameters();
    let query = match query_params.first("query") {
        Some(query) => query.to_lowercase(),
        None => return _400("`query` query param is not found."),
    };

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
            .await?;

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

    let result = json!(candidates
        .iter()
        .map(|item| serde_json::to_value(AttributeValueItem(item.clone())).unwrap())
        .collect::<Vec<serde_json::Value>>())
    .to_string();
    _200(result)
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

    if page <= collect_page_num {
        let start = (page - 1) * (&*PAGE_ITEMS);
        let end = (page) * (&*PAGE_ITEMS);
        posts[start..end].to_vec()
    } else {
        posts[(&*PAGE_ITEMS * collect_page_num)..].to_vec()
    }
}

