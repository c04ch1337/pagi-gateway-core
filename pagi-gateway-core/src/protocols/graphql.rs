use async_graphql::{Context, EmptySubscription, Object, Request as GqlRequest, Schema, Variables};
use hyper::{Body, Method, Request, Response, StatusCode};

use crate::canonical::CanonicalAIRequest;
use crate::registry::AdapterRegistryState;

pub type SchemaType = Schema<QueryRoot, MutationRoot, EmptySubscription>;

pub fn build_schema(registry: AdapterRegistryState) -> SchemaType {
    Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .data(registry)
        .finish()
}

#[derive(serde::Deserialize)]
struct HttpGraphQLRequest {
    query: String,
    #[serde(default)]
    variables: serde_json::Value,
    #[serde(default, rename = "operationName")]
    operation_name: Option<String>,
}

pub async fn handle(req: Request<Body>, schema: SchemaType) -> Result<Response<Body>, hyper::Error> {
    match *req.method() {
        Method::GET => Ok(Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "text/plain; charset=utf-8")
            .body(Body::from(
                "GraphQL endpoint. Send POST /graphql with {query, variables, operationName}.",
            ))
            .unwrap()),
        Method::POST => {
            let body = hyper::body::to_bytes(req.into_body()).await?;
            let parsed: HttpGraphQLRequest = match serde_json::from_slice(&body) {
                Ok(v) => v,
                Err(_) => {
                    return Ok(Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .body(Body::from("invalid graphql http request"))
                        .unwrap());
                }
            };

            let mut gql = GqlRequest::new(parsed.query);
            if let Some(op) = parsed.operation_name {
                gql = gql.operation_name(op);
            }
            if !parsed.variables.is_null() {
                if let Ok(vars) = serde_json::from_value::<Variables>(parsed.variables) {
                    gql = gql.variables(vars);
                }
            }

            let resp = schema.execute(gql).await;
            let out = serde_json::to_vec(&resp).expect("serialize graphql response");
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "application/json")
                .body(Body::from(out))
                .unwrap())
        }
        _ => Ok(Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .body(Body::from("method not allowed"))
            .unwrap()),
    }
}

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn ping(&self) -> &str {
        "pong"
    }
}

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn ai_call(&self, ctx: &Context<'_>, agent_id: String, text: String) -> async_graphql::Result<String> {
        let registry = ctx.data::<AdapterRegistryState>()?;
        let req = CanonicalAIRequest::chat_text(Some(agent_id), text);
        let resp = registry.forward(req).await.map_err(|e| async_graphql::Error::new(e.to_string()))?;
        Ok(resp.json)
    }
}
