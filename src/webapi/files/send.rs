
use actix_web::{
    Error, http::{header::{self, ContentType}}, web::{self, Bytes}, HttpResponse
};
use async_stream::stream;
use log::{debug, error};
use std::{convert::TryInto, fs::File, io::{Read, Seek}, path::PathBuf};

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

async fn send_file_stream(mut file: File, chunk_size: usize)->Result<HttpResponse, Error>{
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
    Ok(HttpResponse::Ok()
        .content_type(ContentType::octet_stream())
        .streaming(file_stream))
}

async fn send_file_chunk(mut file: File, chunk_size: usize)->Result<HttpResponse, Error>{
    let (ch, size) = web::block(move || {
        let mut chunk = vec![0u8;chunk_size];
        file.read_to_end(&mut chunk).map(|s| (chunk, s))
    }).await??;

    Ok(HttpResponse::Ok()
        .content_type(ContentType::json())
        .body(ch[..size].to_vec())
    )
}

pub(super) async fn send_file(conf: &FsConfig, path: PathBuf) -> HttpResponse {
    debug!("Reading file: {:?}",&path);
    let (file, size) = match web::block(move || 
        File::open(path)
        .and_then(|f| 
            f
            .metadata()
            .map(|meta| {
                let size : usize = meta.len().try_into().unwrap();
                (f,size)
            })
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
    let out = if size > conf.max_single_chunk {send_file_stream(file, conf.chunk_size).await}
                                           else {send_file_chunk(file, conf.chunk_size).await};
    match out{
        Ok(res) => res,
        Err(e) => {
            error!("Send file failed: {e}");
            HttpResponse::InternalServerError().finish()
        }
    }
}