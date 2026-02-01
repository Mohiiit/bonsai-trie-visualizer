use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    extract::{Query, State},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use clap::Parser;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};

use bonsai_db_reader as reader;
use bonsai_types::{CfsResponse, DiffResponse, LeafResponse, NodeResponse, ProofResponse, RootResponse, TrieKind};

#[derive(Debug, Parser)]
#[command(name = "bonsai-api")]
struct Args {
    #[arg(long, value_name = "PATH")]
    db_path: Option<String>,

    #[arg(long, default_value_t = 4010)]
    port: u16,
}

#[derive(Clone)]
struct AppState {
    db: Arc<RwLock<Option<reader::db::RocksDb>>>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let db = if let Some(path) = args.db_path.as_deref() {
        reader::open_db(path).ok()
    } else {
        None
    };
    let state = AppState {
        db: Arc::new(RwLock::new(db)),
    };

    let cors = CorsLayer::new().allow_origin(Any).allow_methods(Any);

    let app = Router::new()
        .route("/api/health", get(health))
        .route("/api/open", post(open_db))
        .route("/api/cfs", get(cfs))
        .route("/api/trie/root", get(trie_root))
        .route("/api/trie/node", get(trie_node))
        .route("/api/trie/leaf", get(trie_leaf))
        .route("/api/diff", get(diff_block))
        .route("/api/proof", get(proof))
        .with_state(state)
        .layer(cors);

    let addr = SocketAddr::from(([127, 0, 0, 1], args.port));
    println!("bonsai-api listening on http://{addr}");

    axum::serve(tokio::net::TcpListener::bind(addr).await.unwrap(), app)
        .await
        .unwrap();
}

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({"ok": true}))
}

#[derive(Debug, serde::Deserialize)]
struct OpenQuery {
    db_path: String,
}

async fn open_db(State(state): State<AppState>, Query(params): Query<OpenQuery>) -> impl IntoResponse {
    let mut guard = state.db.write().await;
    match reader::open_db(&params.db_path) {
        Ok(db) => {
            *guard = Some(db);
            Json(serde_json::json!({"ok": true}))
        }
        Err(err) => Json(serde_json::json!({"ok": false, "error": err})),
    }
}

async fn cfs(State(state): State<AppState>) -> impl IntoResponse {
    let guard = state.db.read().await;
    let Some(db) = guard.as_ref() else {
        return Json(CfsResponse { total: 0, names: Vec::new() });
    };
    Json(reader::list_cfs(db))
}

#[derive(Debug, serde::Deserialize)]
struct TrieQuery {
    trie: TrieKind,
    identifier: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct NodeQuery {
    trie: TrieKind,
    identifier: Option<String>,
    path: String,
}

#[derive(Debug, serde::Deserialize)]
struct LeafQuery {
    trie: TrieKind,
    identifier: Option<String>,
    key: String,
}

#[derive(Debug, serde::Deserialize)]
struct DiffQuery {
    trie: TrieKind,
    block: u64,
}

async fn trie_root(State(state): State<AppState>, Query(params): Query<TrieQuery>) -> impl IntoResponse {
    let guard = state.db.read().await;
    let Some(db) = guard.as_ref() else {
        return Json(RootResponse { path_hex: "0x00".to_string(), node: None });
    };
    Json(reader::root_node(db, params.trie, params.identifier))
}

async fn trie_node(State(state): State<AppState>, Query(params): Query<NodeQuery>) -> impl IntoResponse {
    let guard = state.db.read().await;
    let Some(db) = guard.as_ref() else {
        return Json(NodeResponse { path_hex: params.path, node: None });
    };
    Json(reader::load_node(db, params.trie, params.identifier, &params.path))
}

async fn trie_leaf(State(state): State<AppState>, Query(params): Query<LeafQuery>) -> impl IntoResponse {
    let guard = state.db.read().await;
    let Some(db) = guard.as_ref() else {
        return Json(LeafResponse { key: params.key, value: None });
    };
    Json(reader::leaf_value(db, params.trie, params.identifier, &params.key))
}

async fn diff_block(State(state): State<AppState>, Query(params): Query<DiffQuery>) -> impl IntoResponse {
    let guard = state.db.read().await;
    let Some(db) = guard.as_ref() else {
        return Json(DiffResponse { entries: Vec::new() });
    };
    Json(reader::diff_for_block(db, params.trie, params.block))
}

async fn proof(State(state): State<AppState>, Query(params): Query<LeafQuery>) -> impl IntoResponse {
    let guard = state.db.read().await;
    let Some(db) = guard.as_ref() else {
        return Json(ProofResponse { verified: false, nodes: Vec::new() });
    };
    Json(reader::proof_for_key(db, params.trie, params.identifier, &params.key))
}

