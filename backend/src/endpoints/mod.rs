mod ws;

use actix_web::web::ServiceConfig;
use actix_web::{get, web, HttpRequest, HttpResponse, Responder};
use serde_json::json;

#[get("/")]
pub async fn index() -> impl Responder {
    web::Json(json!({
        "ok": true,
    }))
}

#[get("/ws")]
async fn websocket(
    req: HttpRequest,
    stream: web::Payload,
) -> Result<HttpResponse, actix_web::Error> {
    actix_web_actors::ws::start(ws::Runner {}, &req, stream)
}

pub fn configure(svc: &mut ServiceConfig) {
    svc.service(index).service(websocket);
}
