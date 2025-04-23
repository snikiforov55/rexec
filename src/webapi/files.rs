use actix_multipart::Multipart;
use actix_web::{
    web::{self, Data},
    HttpResponse,
};
use log::debug;
use std::{path::PathBuf, sync::Arc};

use crate::util::config::{Config, UrlPathMap};

mod save_multipart;
mod cfg;
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
            .route(web::post().to(
                async move |cfg: Data<Arc<Config>>,
                            req: Multipart,
                            path: web::Path<(String, String)>| {                    
                    match sanitize_path(&cfg.get_ref().fs.entries, &path.0, &path.1) {
                        None => {
                            debug!("Path alias {}{} not found or malformed", path.0, path.1);
                            HttpResponse::NotFound().finish()
                        }
                        Some(path) => match save_multipart::save_file_multipart(
                            &cfg.get_ref().fs, req, path).await{
                            Ok(res) => res,
                            Err(e) => {
                                debug!("Error processing multipart request: {}", &e);
                                HttpResponse::InternalServerError().finish()
                            }
                        },
                    }
                },
            )),
    );
    service_cfg.service(scope);
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, io::Write, path::PathBuf, sync::Arc};

    use actix_web::{
        dev::Service,
        http::{
            header::{HeaderName, HeaderValue},
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
        cfg.fs.entries = HashMap::from([(
            "foo".to_string(),
            dest_path.clone(),
        )]);
        let app = test::init_service(
            App::new()
                .app_data(Data::new(Arc::new(cfg)))
                .configure(configure_files),
        )
        .await;

        let meta = r#"{"create_dir": true, "override_file": true}"#;
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
        let exists = std::fs::exists(file_path).map_err(|_|());
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
}
