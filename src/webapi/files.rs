use actix_multipart::Multipart;
use actix_web::{
    guard::{self, Guard, GuardContext},
    http::header,
    web::{self, Data},
    HttpResponse,
};
use cfg::SaveOptions;
use log::debug;
use mime::Mime;
use std::{path::PathBuf, sync::Arc};

use crate::util::config::{Config, UrlPathMap};

mod cfg;
mod save_multipart;
mod save_single;
pub mod send;

fn sanitize_path(map: &UrlPathMap, url: &String, path: &String) -> Option<PathBuf> {
    match map.get(url) {
        None => {
            debug!("Path alias url {} not found", url);
            None
        }
        Some(dir) => {
            if path.contains("..") {
                debug!("Attempting invalid filename {}", path);
                return None;
            }
            let mut d = dir.clone();
            d.push(path);
            Some(d)
        }
    }
}
struct ContentTypeMultipart;

impl Guard for ContentTypeMultipart {
    fn check(&self, req: &GuardContext) -> bool {
        req.head()
            .headers()
            .get(&header::CONTENT_TYPE)
            .and_then(|h| h.to_str().ok())
            .and_then(|v| v.parse::<Mime>().ok())
            .map(|mime| mime.type_() == mime::MULTIPART)
            .unwrap_or(false)
    }
}

pub(super) fn configure_files(service_cfg: &mut web::ServiceConfig) {
    let scope = web::scope("/fs").service(
        web::resource(format!("{{alias}}/{{file}}*"))
            .route(web::get().to(
                async move |cfg: Data<Arc<Config>>, index: web::Path<(String, String)>| {
                    match sanitize_path(&cfg.get_ref().fs.entries, &index.0, &index.1) {
                        None => {
                            debug!("Path alias {} not found", &index.0);
                            HttpResponse::NotFound().finish()
                        }
                        Some(path) => send::send_file(&cfg.get_ref().fs, path).await,
                    }
                },
            ))
            .route(
                web::route()
                    .guard(guard::Post())
                    .guard(ContentTypeMultipart)
                    .to(
                        async move |cfg: Data<Arc<Config>>,
                                    req: Multipart,
                                    path: web::Path<(String, String)>| {
                            match sanitize_path(&cfg.get_ref().fs.entries, &path.0, &path.1) {
                                None => {
                                    debug!(
                                        "Path alias {}{} not found or malformed",
                                        path.0, path.1
                                    );
                                    HttpResponse::NotFound().finish()
                                }
                                Some(path) => save_multipart::save_file_multipart(
                                    &cfg.get_ref().fs,
                                    req,
                                    path,
                                )
                                .await
                                .map_err(|e| {
                                    debug!("Error processing multipart request: {}", &e);
                                    ()
                                })
                                .unwrap_or(HttpResponse::InternalServerError().finish()),
                            }
                        },
                    ),
            )
            .route(web::route().guard(guard::Post()).to(
                async move |cfg: Data<Arc<Config>>,
                            req: web::Payload,
                            path: web::Path<(String, String)>,
                            query: Option<web::Query<SaveOptions>>| {
                    match sanitize_path(&cfg.get_ref().fs.entries, &path.0, &path.1) {
                        None => {
                            debug!("Path alias {}{} not found or malformed", path.0, path.1);
                            HttpResponse::NotFound().finish()
                        }
                        Some(path) => save_single::save_file_single(&cfg.get_ref().fs,req, path, query)
                            .await
                            .map_err(|e| {
                                debug!("Error processing single request: {}", &e);
                                ()
                            })
                            .unwrap_or(HttpResponse::InternalServerError().finish()),
                    }
                },
            )),
    );
    service_cfg.service(scope);
}

#[cfg(test)]
mod tests {
     #[test]
    fn test_borrow() {
        struct Data {
            a: i32,
            b: String,
        }
        fn use_f(f: Data) -> Data {
            print!("{}{}", f.a, f.b);
            f
        }
        let mut d = Data {
            a: 0,
            b: "0".to_string(),
        };
        d = use_f(d);
        use_f(d);
    }
}
