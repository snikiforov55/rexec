
use actix_web::{
    http::{header::{self, ContentType}, Error},
    web::{self, Bytes},
    HttpResponse,
};
use async_stream::stream;
use log::{debug, error};
use std::{fs::File, io::Read, path::PathBuf};

use crate::{
    error::{RexecError, RexecErrorType},
    util::config::FsConfig,
};

pub(crate) async fn list_log_files(
    index: web::Path<(String, String)>,
) -> Result<HttpResponse, Error> {
    let (_, i) = index.into_inner();
    debug!("list_log_files {}", i);
    Ok(HttpResponse::Ok().finish())
}

pub(crate) async fn nope() -> HttpResponse {
    debug!("nope");
    HttpResponse::Ok().finish()
}

pub(super) async fn send_file(conf: &FsConfig, path: PathBuf) -> HttpResponse {
    debug!("Reading file: {:?}",&path);
    let (mut file, size) = match web::block(move || 
        File::open(path)
        .and_then(|f| 
            f
            .metadata()
            .map(|meta| (f,meta.len()))
        )
    ).await {
        Err(e) => {
            error!("Failed to start web::block, error: {}", e);
            return HttpResponse::InternalServerError().finish();
        }
        Ok(Err(e)) => {
            debug!("Failed to open file, error: {}", e);
            return HttpResponse::Forbidden().finish();
        }
        Ok(Ok(f)) => f,
    };
    let chunk_size = conf.chunk_size;
    let file_stream = stream! {
        let mut chunk = vec![0u8;chunk_size];
        loop{
            match web::block(move || file.read(&mut chunk).map(|n| (file, n, chunk))).await{
            Err(e) => {
                error!("Error executing web::block: {}", e);
                yield Result::<Bytes, RexecError>::Err(RexecError { code: RexecErrorType::FailedFileRead, message:e.to_string()});
                break;
            },
            Ok(Err(e))=>{
                debug!("Error reading file: {}", e);
                yield Result::<Bytes, RexecError>::Err(RexecError { code: RexecErrorType::FailedFileRead, message:e.to_string()});
                break;
            }
            Ok(Ok((_,0, _)))=>break, // End of file
            Ok(Ok((f,n,c)))=>{
                file = f;
                chunk = c;
                yield Result::<Bytes, RexecError>::Ok(Bytes::from(chunk[..n].to_vec())); // Yielding the chunk here
            },
            }
        }
    };
    HttpResponse::Ok()
        .content_type(ContentType::octet_stream())
        .streaming(file_stream)
}