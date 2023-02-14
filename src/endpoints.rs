use crate::{
    order_book::{Event, OrderBookState},
    AppContext, Error, Result,
};

use axum::{
    debug_handler, extract::Path, http::StatusCode, response::IntoResponse, response::Response,
    routing::get, routing::patch, routing::post, Extension, Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Internal Server Error: {}", self.0),
        )
            .into_response()
    }
}

async fn health_check() -> String {
    // TODO check database connectivity
    "OK".to_string()
}

pub fn routes() -> Router {
    Router::new()
        .route("/health-check", get(health_check))
        .merge(routes_v1())
}

fn routes_v1() -> Router {
    Router::new().nest("/api/v1", order_book_routes())
}

fn order_book_routes() -> Router {
    // GET v1/order-book/ returns the state of buy/sell book
    // POST v1/order-book/buy submit a buy order (returns Uuid of the order)
    // POST v1/order-book/sell submit a sell order (returns Uuid of the order)
    // PATCH v1/order-book/buy/{uuid} updates a buy order with new price and quantity
    // PATCH v1/order-book/sell/{uuid} updates a sell order with new price and quantity
    // DELETE v1/order-book/buy/{uuid} cancel a buy order
    // DELETE v1/order-book/sell/{uuid} cancel a sell order
    Router::new()
        .route("/order-book", get(get_order_book))
        .route("/order-book/sell", post(post_sell))
        .route(
            "/order-book/sell/:id",
            patch(patch_sell).delete(delete_sell),
        )
        .route("/order-book/buy", post(post_buy))
        .route("/order-book/buy/:id", patch(patch_buy).delete(delete_buy))
}

#[derive(Serialize)]
struct EventsResponse {
    events: Vec<Event>,
}

#[derive(Deserialize)]
struct OrderRequest {
    quantity: u32,
    price: u32,
}

#[debug_handler()]
async fn get_order_book(
    Extension(app_context): Extension<AppContext>,
) -> Result<Json<OrderBookState>> {
    let state = app_context.actor_client.get_order_book().await?;
    Ok(Json(state))
}

#[debug_handler()]
async fn post_buy(
    Extension(app_context): Extension<AppContext>,
    Json(OrderRequest { quantity, price }): Json<OrderRequest>,
) -> Result<Json<EventsResponse>> {
    let events = app_context.actor_client.buy(quantity, price).await?;
    Ok(Json(EventsResponse { events }))
}

#[debug_handler()]
async fn patch_buy(
    Extension(app_context): Extension<AppContext>,
    Path(id): Path<Uuid>,
    Json(OrderRequest { quantity, price }): Json<OrderRequest>,
) -> Result<Json<EventsResponse>> {
    let events = app_context.actor_client.update(id, quantity, price).await?;
    Ok(Json(EventsResponse { events }))
}

#[debug_handler()]
async fn delete_buy(
    Extension(app_context): Extension<AppContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<EventsResponse>> {
    let events = app_context.actor_client.cancel(id).await?;
    Ok(Json(EventsResponse { events }))
}

#[debug_handler()]
async fn post_sell(
    Extension(app_context): Extension<AppContext>,
    Json(OrderRequest { quantity, price }): Json<OrderRequest>,
) -> Result<Json<EventsResponse>> {
    let events = app_context.actor_client.sell(quantity, price).await?;
    Ok(Json(EventsResponse { events }))
}

#[debug_handler()]
async fn patch_sell(
    Extension(app_context): Extension<AppContext>,
    Path(id): Path<Uuid>,
    Json(OrderRequest { quantity, price }): Json<OrderRequest>,
) -> Result<Json<EventsResponse>> {
    let events = app_context.actor_client.update(id, quantity, price).await?;
    Ok(Json(EventsResponse { events }))
}

#[debug_handler()]
async fn delete_sell(
    Extension(app_context): Extension<AppContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<EventsResponse>> {
    let events = app_context.actor_client.cancel(id).await?;
    Ok(Json(EventsResponse { events }))
}
