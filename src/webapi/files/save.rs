use actix_multipart::{Field, Multipart, MultipartError};
use actix_web::{web, HttpResponse};
use futures_util::{FutureExt, StreamExt, TryFutureExt, TryStreamExt};
use log::{debug, error};
use serde::Deserialize;
use std::{fs::File, io::Write, path::PathBuf};

use crate::{
    error::{RexecError, RexecErrorType},
    util::config::FsConfig,
};

#[derive(Deserialize, Clone, Debug)]
struct SaveOptions {
    create_dir: Option<bool>,
    override_file: Option<bool>,
}
impl SaveOptions {
    fn default() -> SaveOptions {
        Self {
            create_dir: Some(true),
            override_file: Some(false),
        }
    }
}

async fn write_chunks(mut field: Field, mut file: File) -> Result<File, RexecError> {
    while let Some(res) = field.next().await {
        match res {
            Ok(chunk) => {
                match web::block(move || {
                    let res = file.write_all(&chunk);
                    (file, res)
                })
                .await
                {
                    Ok((f, Ok(_))) => {
                        // return moved file to the scope here
                        file = f;
                    }
                    Ok((_, Err(e))) => {
                        return Err(RexecError {
                            code: RexecErrorType::FailedFileWrite,
                            message: e.to_string(),
                        })
                    }
                    Err(e) => {
                        return Err(RexecError {
                            code: RexecErrorType::FailedFileWrite,
                            message: e.to_string(),
                        })
                    }
                }
            }
            Err(e) => {
                return Err(RexecError {
                    code: RexecErrorType::FailedFileWrite,
                    message: e.to_string(),
                })
            }
        }
    }
    Ok(file)
}

pub(super) async fn save_file(
    conf: &FsConfig,
    mut mp: Multipart,
    path: PathBuf,
) -> Result<HttpResponse, MultipartError> {
    debug!("Saving file: {:?}", &path);
    println!("Saving file: {:?}", &path);

    let (field, config) = match mp.try_next().await? {
        None => return Err(MultipartError::Incomplete),
        Some(mut field) => {
            if field
                .name()
                .map(|name| if name == "meta" { true } else { false })
                .unwrap_or(false)
            {
                if let Ok(bytes) = field
                    .bytes(conf.metadata_limit)
                    .await
                    .map_err(|_| MultipartError::NotConsumed)?
                {
                    match serde_json::from_slice::<SaveOptions>(&bytes[..]) {
                        Ok(cfg) => (None, cfg),
                        _ => (None, SaveOptions::default()),
                    }
                } else {
                    return Err(MultipartError::NotConsumed);
                }
            } else {
                (Some(field), SaveOptions::default())
            }
        }
    };
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
    let fo = web::block(move || {
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
    })?;
    match fo {
        Ok(mut file) => {
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
        }
        Err(e) => {
            error!("Failed create file for writing {e}");
            return Err(MultipartError::NotConsumed);
        }
    }
    //reply
    Ok(HttpResponse::Ok().finish())
}
