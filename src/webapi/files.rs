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
    use std::{
        collections::HashMap,
        fs::File,
        io::{Read, Write},
        path::PathBuf,
        sync::Arc,
    };
    use actix_web::{
        dev::Service,
        http::{
            header::{self, ContentType, HeaderName, HeaderValue},
            StatusCode,
        },
        test,
        web::Data,
        App,
    };

    use crate::util::config::Config;

    use super::configure_files;

    fn build_multipart_payload_and_header(
        chunks: Vec<(&str, &str)>,
    ) -> (Vec<u8>, (HeaderName, HeaderValue)) {
        let boundary = "-----------------------------202022185716362916172375148227";
        let mut out: Vec<u8> = vec![];
        out.reserve(1024);

        for (name, payload) in chunks {
            out.write_all(
                format!(
                    "{boundary}\r\n\
                Content-Disposition: form-data; name=\"{name}\"\r\n\
                Content-Type: text/csv\r\n\
                \r\n\r\n\
                {payload}\r\n\r\n\
                "
                )
                .as_bytes(),
            )
            .ok();
        }
        out.write_all(format!("{boundary}--").as_bytes());

        let header = (
            actix_web::http::header::CONTENT_TYPE,
            HeaderValue::from_static("multipart/form-data; boundary=---------------------------202022185716362916172375148227"),
        );
        (out, header)
    }

    #[actix_web::test]
    async fn test_upload_and_override_file() {
        let mut cfg = Config::new();
        let id = uuid::Uuid::new_v4();
        let dest_path = PathBuf::from(format!("/tmp/rexec/test_{id}"));
        cfg.fs.entries = HashMap::from([("foo".to_string(), dest_path.clone())]);
        let app = test::init_service(
            App::new()
                .app_data(Data::new(Arc::new(cfg)))
                .configure(configure_files),
        )
        .await;

        let meta = r#"{"create_dir": true, "replace_file": true}"#;
        let file = r#"{"test": "file", "example": true}"#;

        let (payload, header) =
            build_multipart_payload_and_header(vec![("meta", meta), ("file", file)]);

        let file_name = "test.json";
        let alias = "foo";
        let req = test::TestRequest::post()
            .uri(format!("/fs/{alias}/{file_name}").as_str())
            .insert_header(header.clone())
            .set_payload(payload.clone())
            .to_request();
        let resp = app.call(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);

        let mut file_path = dest_path.clone();
        file_path.push(file_name);
        let exists = std::fs::exists(file_path).map_err(|_| ());
        assert_eq!(exists, Ok(true));

        let req = test::TestRequest::post()
            .uri(format!("/fs/{alias}/{file_name}").as_str())
            .insert_header(header)
            .set_payload(payload)
            .to_request();
        let resp = app.call(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
    #[test]
    async fn test_borrow() {
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
    #[actix_web::test]
    async fn test_upload_and_override_single_file() {
        let mut cfg = Config::new();
        let id = uuid::Uuid::new_v4();
        let dest_path = PathBuf::from(format!("/tmp/rexec/test_{id}"));
        cfg.fs.entries = HashMap::from([("foo".to_string(), dest_path.clone())]);
        let app = test::init_service(
            App::new()
                .app_data(Data::new(Arc::new(cfg)))
                .configure(configure_files),
        )
        .await;

        let file = r#"{"test": "file", "example": true}"#;

        let file_name = "test.json";
        let alias = "foo";
        let req = test::TestRequest::post()
            .uri(format!("/fs/{alias}/{file_name}?create_dir=true").as_str())
            .insert_header((header::CONTENT_TYPE, ContentType::plaintext()))
            .insert_header((header::CONTENT_LENGTH, file.len()))
            .set_payload(file.as_bytes())
            .to_request();
        let resp = app.call(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);

        let mut file_path = dest_path.clone();
        file_path.push(file_name);
        let exists = std::fs::exists(&file_path).map_err(|_| ());
        assert_eq!(exists, Ok(true));

        let file_content = File::open(&file_path).and_then(|mut f| {
            let mut bytes = vec![];
            f.read_to_end(&mut bytes).map(|_| bytes)
        });
        assert_eq!(file_content.unwrap(), file.as_bytes());

        let req = test::TestRequest::post()
            .uri(format!("/fs/{alias}/{file_name}").as_str())
            .insert_header((header::CONTENT_TYPE, ContentType::plaintext()))
            .insert_header((header::CONTENT_LENGTH, file.len()))
            .set_payload(file.as_bytes())
            .to_request();
        let resp = app.call(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
