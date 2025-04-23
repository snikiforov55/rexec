use actix_multipart::{Field, Multipart, MultipartError};
use actix_web::{web, HttpResponse};
use futures_util::{StreamExt, TryStreamExt};
use log::{debug, error};
use std::{fs::File, io::Write, path::PathBuf};

use crate::{
    error::{RexecError, RexecErrorType},
    util::config::FsConfig,
    webapi::files::cfg::SaveOptions,
};

async fn write_chunks(mut field: Field, mut file: File) -> Result<File, RexecError> {
    while let Some(chunk) = field.try_next().await.map_err(|e| {
        debug!("Failed next chunk: {e}");
        RexecError::code(RexecErrorType::FailedFileWrite)
    })? {
        let (f, r) = web::block(move || {
            let res = file.write_all(&chunk);
            (file, res)
        })
        .await
        .map_err(|e| {
            debug!("Failed next chunk: {e}");
            RexecError::code(RexecErrorType::FailedFileWrite)
        })?;
        file = f;

        if let Err(e) = r {
            return Err(RexecError {
                code: RexecErrorType::FailedFileWrite,
                message: e.to_string(),
            });
        }
    }
    Ok(file)
}

async fn metadata(
    of: Option<Field>,
    limit: usize,
) -> Result<(Option<Field>, SaveOptions), MultipartError> {
    match of {
        None => return Err(MultipartError::Incomplete),
        Some(mut field) => {
            if field
                .name()
                .map(|name| if name == "meta" { true } else { false })
                .unwrap_or(false)
            {
                if let Ok(bytes) = field
                    .bytes(limit)
                    .await
                    .map_err(|_| MultipartError::NotConsumed)?
                {
                    match serde_json::from_slice::<SaveOptions>(&bytes[..]) {
                        Ok(cfg) => Ok((None, cfg)),
                        _ => Ok((None, SaveOptions::default())),
                    }
                } else {
                    return Err(MultipartError::NotConsumed);
                }
            } else {
                Ok((Some(field), SaveOptions::default()))
            }
        }
    }
}

pub(super) async fn save_file_multipart(
    conf: &FsConfig,
    mut mp: Multipart,
    path: PathBuf,
) -> Result<HttpResponse, MultipartError> {
    debug!("Saving file: {:?}", &path);

    let (field, config) = metadata(mp.try_next().await?, conf.metadata_limit).await?;

    // create directory
    if config.create_dir.unwrap_or(false) {
        let p = path.clone();
        web::block(move || {
            let path_parent = p.parent().unwrap_or(std::path::Path::new(""));
            std::fs::create_dir_all(path_parent)
        })
        .await
        .map_err(|e| {
            error!("Failed to spawn web::block {e}");
            MultipartError::Incomplete
        })?
        .map_err(|e| {
            debug!("Failed create directory: {e}");
            MultipartError::NotConsumed
        })?;
    }
    // open file for writing
    let path_ref = path.clone();
    let mut file = web::block(move || {
        File::options()
            .truncate(true)
            .write(true)
            .create(true)
            .create_new(!config.override_file.unwrap_or(false))
            .open(path_ref.as_path())
    })
    .await
    .map_err(|e| {
        error!("Failed spawn web::block {e}");
        MultipartError::NotConsumed
    })?
    .map_err(|e| {
        debug!("Failed Open/Create File {e}");
        MultipartError::NotConsumed
    })?;

    if let Some(field) = field {
        file = write_chunks(field, file)
            .await
            .map_err(|_| MultipartError::NotConsumed)?;
    }
    // write content
    while let Some(field) = mp.try_next().await? {
        file = write_chunks(field, file)
            .await
            .map_err(|_| MultipartError::NotConsumed)?;
    }
    let _ = file.flush();

    //reply
    Ok(HttpResponse::Ok().finish())
}
