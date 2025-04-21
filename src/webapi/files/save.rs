
use actix_multipart::{Field, Multipart, MultipartError};
use actix_web::{
    web,
    HttpResponse,
};
use futures_util::{FutureExt, StreamExt, TryFutureExt, TryStreamExt};
use log::{debug,error};
use serde::Deserialize;
use std::{fs::File, io::Write, path::PathBuf};

use crate::{
    error::{RexecError, RexecErrorType},
    util::config::FsConfig,
};


#[derive(Deserialize, Clone, Debug)]
struct SaveOptions{
    create_dir: Option<bool>,
    override_file: Option<bool>,
}
impl SaveOptions{
    fn default()->SaveOptions{
        Self{create_dir: Some(true), override_file: Some(false)}
    }
}

async fn write_chunks(mut field: Field, mut file: File) -> Result<File, RexecError>{
    while let Some(res) = field.next().await {
        match res{
        Ok(chunk) => {
            match web::block(move || {let res = file.write_all(&chunk);(file, res)}).await{
            Ok((f, Ok(_))) => {
                // return moved file to the scope here
                file = f;
            },
            Ok((_, Err(e))) => return Err(RexecError{code: RexecErrorType::FailedFileWrite, message: e.to_string()}),
            Err(e) => return Err(RexecError{code: RexecErrorType::FailedFileWrite, message: e.to_string()})
            }
        },
        Err(e) => return Err(RexecError{code: RexecErrorType::FailedFileWrite, message: e.to_string()})
        }
    }
    Ok(file)
}

pub(super) async fn save_file(conf: &FsConfig, mut mp: Multipart, path: PathBuf)->HttpResponse{

    debug!("Saving file: {:?}", &path);
    // let first_chunk = mp
    //     .try_next()
    //     .and_then(|of| async{match of{
    //             Some(f) => Ok(f),
    //             _ => Err(MultipartError::Incomplete)
    //         }
    //     })
    //     .and_then(|f| async{match f.name(){
    //         Some(name) => Ok((f,name.to_string())),
    //         _ => Err(MultipartError::Incomplete)
    //     }})
    //     .and_then(|(f,n)| async{
    //         if n == "meta"{Ok((f, Some(f.bytes(conf.metadata_limit))))}
    //         else {Ok((f,None))}
    //     })
    //     .await;//.unwrap_or((None,None));

    let first_chunk = match mp.try_next().await{
        Ok(Some(mut field)) => {
            debug!("Field: {:?}",&field);
            let c = match field.name(){
                Some(n) => if n == "meta"{
                    if let Ok(Ok(bytes)) = field.bytes(conf.metadata_limit).await{
                        match serde_json::from_slice::<SaveOptions>(&bytes[..]){
                            Ok(cfg) => Some(cfg),
                            _ => Some(SaveOptions::default())
                        }
                    }
                    else {Some(SaveOptions::default())}
                }else{None},
                None => None
            };
            match c{
                Some(c) => Some((None, Some(c))),
                _ => Some((Some(field), None))
            }
        }
        _ => None
    };
    debug!("Got first chunk: {:?}", first_chunk);   
    let (config, field) = match first_chunk{
        Some((f,Some(c))) => (c,f),
        Some((f, None)) => (SaveOptions::default(),f),
        _ => return HttpResponse::NoContent().finish()
    };

    // create directory
    if config.create_dir.unwrap_or(false){
        let path = path.clone();
        if let Err(e) = web::block(move || std::fs::create_dir_all(path.parent().unwrap_or(std::path::Path::new("")))).await{
            error!("Failed to spawn web::block {e}");
            return HttpResponse::InternalServerError().finish()
        }
    }
    // open file for writing
    let path_ref = path.clone();
    if let Ok(Ok(mut file)) = web::block(move || 
        File::options()
        .truncate(true)
        .create_new(!config.override_file.unwrap_or(false))
        .open(path_ref.as_path())
        ).await{

        if let Some(field) = field{
            match write_chunks(field, file).await {
                Ok(f) => file = f,
                Err(_) => return HttpResponse::InternalServerError().finish()
            }
        }

            // write content
        // let mut field = first_chunk.unwrap().0;
        // while  {

        // }
    // flush file, close file
    }
    else{
        debug!("Failed to open local file {:?} for witing ", path);
        return HttpResponse::InternalServerError().finish()
    }
    //reply
    HttpResponse::Ok().finish()
}
