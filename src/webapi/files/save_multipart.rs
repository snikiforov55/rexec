use actix_multipart::{Field, Multipart, MultipartError};
use actix_web::{error::ErrorInternalServerError, web, Error, HttpResponse};
use futures_util::TryStreamExt;
use log::{debug, error};
use std::{fs::File, io::Write, path::PathBuf};

use crate::{
    util::config::FsConfig,
    webapi::files::cfg::SaveOptions,
};

async fn write_chunks(mut field: Field, mut file: File, mut saved_bytes: usize, max_file: usize) -> Result<(File, usize), Error> {
    while let Some(chunk) = field.try_next().await? {
        saved_bytes += chunk.len();
        if saved_bytes > max_file {

            error!("Multipart Payload size is too large");
            return Err(ErrorInternalServerError(""))
        }
        let (f, r) = web::block(move || {
            let res = file.write_all(&chunk);
            (file, res)
        }).await?;
        file = f;
        
        if let Err(e) = r {return Err(e.into())}
    }
    Ok((file, saved_bytes))
}

async fn metadata(mut field: Field, limit: usize) -> Result<(Option<Field>, SaveOptions), Error> {
    if field
        .name()
        .map(|name| if name == "meta" { true } else { false })
        .unwrap_or(false)
    {
        let bytes = field.bytes(limit).await.map_err(|_| MultipartError::Incomplete)??;
        match serde_json::from_slice::<SaveOptions>(&bytes[..]) {
            Ok(cfg) => Ok((None, cfg)),
            _ => Ok((None, SaveOptions::default())),
        }
    } else {
        Ok((Some(field), SaveOptions::default()))
    }
}

pub(super) async fn save_file_multipart(conf: &FsConfig,mut mp: Multipart,path: PathBuf) -> Result<HttpResponse, Error> {
    debug!("Saving file: {:?}", &path);

    let (field, config) = if let Some(field) = mp.try_next().await?{
        metadata(field, conf.metadata_limit).await?
    }
    else{
        return Err(MultipartError::Incomplete.into())
    };

    // create directory
    if config.create_dir.unwrap_or(false) {
        let p = path.clone();
        web::block(move || {
            let path_parent = p.parent().unwrap_or(std::path::Path::new(""));
            std::fs::create_dir_all(path_parent)
        })
        .await??;
    }
    // open file for writing
    let path_ref = path.clone();
    let mut file = web::block(move || {
        File::options()
            .truncate(true)
            .write(true)
            .create(true)
            .create_new(!config.replace_file.unwrap_or(false))
            .open(path_ref.as_path())
    })
    .await??;

    let mut saved_bytes :usize = 0;
    if let Some(field) = field {
        let (f, s) = write_chunks(field, file, saved_bytes, conf.max_file).await?;
        file = f;
        saved_bytes = s;
    }
    // write content
    while let Some(field) = mp.try_next().await? {
        let (f,s) = write_chunks(field, file, saved_bytes, conf.max_file).await?;
        file = f;
        saved_bytes = s;
    }
    let _ = file.flush();
    //reply
    Ok(HttpResponse::Ok().finish())
}


#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        io::Write,
        path::PathBuf,
        sync::Arc,
    };
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

    use crate::webapi::files::configure_files;

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
        out.write_all(format!("{boundary}--").as_bytes()).ok();

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
        let dest_path = format!("/tmp/rexec/test_{id}");
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

        let mut file_path = PathBuf::from(dest_path.clone());
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
}
