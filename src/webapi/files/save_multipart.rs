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
