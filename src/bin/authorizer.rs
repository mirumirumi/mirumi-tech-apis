use std::env;

use anyhow::{Error, Ok, Result};
use lambda_runtime::{service_fn, LambdaEvent};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[rustfmt::skip]
static UNHASHED_KEY: Lazy<String> = Lazy::new(|| env::var("UNHASHED_KEY").expect("\"UNHASHED_KEY\" env var is not set."));

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Request {
    authorization_token: String,
    method_arn: String,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct PolicyStatement {
    principal_id: String,
    policy_document: PolicyDocument,
}

#[derive(Deserialize, Serialize)]
#[allow(non_snake_case)]
struct PolicyDocument {
    Version: String,
    Statement: Vec<Statement>,
}

#[derive(Deserialize, Serialize)]
#[allow(non_snake_case)]
struct Statement {
    Action: String,
    Effect: String,
    Resource: String,
}

async fn lambda_handler(event: LambdaEvent<Request>) -> Result<PolicyStatement, Error> {
    let payload = event.payload;
    println!("{:?}", payload);

    let mut hasher = Sha256::new();
    hasher.update(&*UNHASHED_KEY.as_bytes());
    let hash = hasher.finalize();

    if payload.authorization_token == hex::encode(hash) {
        return Ok(result("Allow", payload));
    }

    Ok(result("Deny", payload))
}

fn result(effect: &str, event: Request) -> PolicyStatement {
    PolicyStatement {
        principal_id: "*".to_string(),
        policy_document: PolicyDocument {
            Version: "2012-10-17".to_string(),
            Statement: vec![Statement {
                Action: "execute-api:Invoke".to_string(),
                Effect: effect.to_string(),
                Resource: event.method_arn,
            }],
        },
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    lambda_runtime::run(service_fn(lambda_handler))
        .await
        .unwrap();

    Ok(())
}
