use actix_web::{Error, HttpResponse, web};
use log::debug;

pub(super) async fn list_log_files(index: web::Path<(String,String)>) -> Result<HttpResponse,Error> {
    let (_, i) = index.into_inner();
    debug!("list_log_files {}", i);
    Ok(HttpResponse::Ok().finish())
}

pub(super) async fn nope() -> HttpResponse {
    debug!("nope");
    HttpResponse::Ok().finish()
}
