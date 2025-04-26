use std::path::PathBuf;

use actix_web::{Error, HttpRequest, HttpResponse};

use crate::util::config::FsConfig;

pub(super) async fn save_file_single(
    conf: &FsConfig,
    mut req: HttpRequest,
    path: PathBuf,
) -> Result<HttpResponse, Error> {
    Ok(HttpResponse::Ok().finish())
}