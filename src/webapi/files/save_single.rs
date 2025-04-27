use std::{fs::File, io::Write, path::PathBuf};

use actix_web::{error::ErrorInternalServerError, web, Error, HttpResponse};
use futures_util::StreamExt;
use log::{debug, error};

use crate::{util::config::FsConfig, webapi::files::cfg::SaveOptions};

pub(super) async fn save_file_single(
    conf: &FsConfig,
    mut req: web::Payload,
    path: PathBuf,
    config: Option<web::Query<SaveOptions>>,
) -> Result<HttpResponse, Error> {

    let config = config
        .map(|c|c.into_inner())
        .unwrap_or(SaveOptions::default());


    debug!("Saving file from a single shot request: {:?}, save config: {:?}", &path, &config);

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
            .create_new(!config.replace_file.unwrap_or(true))
            .open(path_ref.as_path())
    })
    .await??;

    let mut saved_bytes: usize = 0;
    while let Some(chunk) = req.next().await {
        let chunk = chunk?;
        saved_bytes += chunk.len();
        if saved_bytes > conf.max_file {
            drop(file);
            error!("Payload it too large for file {:?}", &path);
            return Err(ErrorInternalServerError(""))
        }
        let (f, r) = web::block(move || {
            let res = file.write_all(&chunk);
            (file, res)
        })
        .await?;
        let _ = r?;
        file = f;
    }
    let _ = file.flush();
    //reply
    Ok::<HttpResponse, Error>(HttpResponse::Ok().finish())
}


#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        fs::File,
        io::Read,
        path::PathBuf,
        sync::Arc,
    };
    use actix_web::{
        dev::Service,
        http::{
            header::{self, ContentType},
            StatusCode,
        },
        test,
        web::Data,
        App,
    };

    use crate::util::config::Config;
    use crate::webapi::files::configure_files;

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
