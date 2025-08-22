use axum::{
    extract::{State, Path, Query},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use crate::{AppState};
use crate::model::Link;
use crate::services::llmdb::LLMDBLink;
use crate::services::queue;
use crate::services::queue::Task;

#[derive(Deserialize)]
pub struct SearchQuery {
    query: String,
}

pub async fn add_link(
    State(state): State<AppState>,
    Json(link): Json<Link>,
) -> Result<Json<String>, StatusCode> {

    let new_id = state.db.insert_link(&link).await.map_err(|e| {
        eprintln!("{:?}",e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let llmd_link = LLMDBLink{link: link.clone(), id: new_id.clone()};
    state.llmdb.insert_link(&llmd_link).await.map_err(|e| {
        eprintln!("{:?}",e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    
    if let Some(queue) = state.queue {
        if let Err(e) = queue::push_task(queue::Task{id: new_id.clone(), payload: "".to_string()}, queue).await {
            eprintln!("{:?}",e);
        }
    } else {
        println!("Add redis service to run messages");
    }
    Ok(Json(new_id.clone()))
}

pub async fn edit_link(
    State(state): State<AppState>,
    Json(llmdlink): Json<LLMDBLink>,
) -> Result<Json<Link>, StatusCode>{

    let option = state.db.edit_link(llmdlink.id.as_str(), &llmdlink.link).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if let Some(new_link) = option {
        let option_llmdb = state.llmdb.edit_link(llmdlink.id.as_str(), &llmdlink).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if let Some(_) = option_llmdb {
            Ok(Json(new_link))
        } else {
            Err(StatusCode::NOT_FOUND)
        }

    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

pub async fn delete_link(
    State(state): State<AppState>,
    Json(id): Json<String>,
) -> Result<Json<()>, StatusCode>{

    let _ = state.db.delete_link(&id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let _ = state.llmdb.delete_link(&id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(()))
}

pub async fn search_links(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<Vec<Link>>, StatusCode> {
    let llmdb_vals = state.llmdb.query(params.query.as_str()).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    //TODO we need to add more metadata here?
    let id_links: Vec<String> = llmdb_vals.iter().map( |llmdb_link| {llmdb_link.id.clone()}).collect();
    let links = state.db.get_links(id_links.as_slice()).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(links))
}
