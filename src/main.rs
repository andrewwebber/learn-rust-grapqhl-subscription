use actix_web::{guard, web, App, HttpRequest, HttpResponse, HttpServer, Result};
use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
use async_graphql::Schema;
use async_graphql::*;
use async_graphql_actix_web::{Request, Response, WSSubscription};
use async_stream::stream;
use futures::lock::Mutex;
use futures::*;
use futures::{Stream, StreamExt};
use futures_util::pin_mut;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::delay_for;

type AppSchema = Schema<Query, EmptyMutation, Subscription>;

struct Query;

#[Object]
impl Query {
    /// Returns the sum of a and b
    async fn add(&self, a: i32, b: i32) -> i32 {
        a + b
    }
}

struct Subscription;

#[Subscription]
impl Subscription {
    async fn integers(&self, #[graphql(default = 1)] step: i32) -> impl Stream<Item = i32> {
        stream! {
            for i in 0..10{
                delay_for(Duration::from_secs(2)).await;
                println!("integer {}", i);
                yield i;
            }
        }
    }
}

async fn index(schema: web::Data<AppSchema>, gql_request: Request) -> Response {
    let query = gql_request.into_inner();
    schema.execute(query).await.into()
}

async fn index_ws(
    schema: web::Data<AppSchema>,
    req: HttpRequest,
    payload: web::Payload,
) -> Result<HttpResponse> {
    WSSubscription::start(Schema::clone(&*schema), &req, payload)
}

async fn index_playground() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(playground_source(
            GraphQLPlaygroundConfig::new("/").subscription_endpoint("/"),
        )))
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    let schema = AppSchema::build(Query, EmptyMutation, Subscription).finish();

    println!("Playground: http://localhost:8000");

    HttpServer::new(move || {
        App::new()
            .data(schema.clone())
            .service(web::resource("/").guard(guard::Post()).to(index))
            .service(
                web::resource("/")
                    .guard(guard::Get())
                    .guard(guard::Header("upgrade", "websocket"))
                    .to(index_ws),
            )
            .service(web::resource("/").guard(guard::Get()).to(index_playground))
    })
    .bind("127.0.0.1:8000")?
    .run()
    .await
}
