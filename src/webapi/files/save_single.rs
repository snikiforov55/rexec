use std::{fs::File, io::Write, path::PathBuf};

use actix_web::{web, Error, HttpResponse};
use futures_util::StreamExt;
use log::debug;

use crate::webapi::files::cfg::SaveOptions;

pub(super) async fn save_file_single(
    mut req: web::Payload,
    path: PathBuf,
    config: web::Query<SaveOptions>,
) -> Result<HttpResponse, Error> {

    let config = config.into_inner();

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
            .create_new(!config.replace_file.unwrap_or(false))
            .open(path_ref.as_path())
    })
    .await??;

    while let Some(chunk) = req.next().await {
        let chunk = chunk?;
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
