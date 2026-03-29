use anyhow::Result;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    http::StatusCode,
    response::Response,
    routing::{delete, get, post, Router},
    Json,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    net::SocketAddr,
    path::PathBuf,
    sync::Arc,
};
use tokio::sync::broadcast;
use tower_http::{cors::CorsLayer, services::ServeDir};

use crate::graph::Graph;
use crate::scan;

// Shared state for the server
#[derive(Clone)]
struct AppState {
    root: PathBuf,
    graph: Arc<tokio::sync::RwLock<Graph>>,
    ws_tx: broadcast::Sender<WsMessage>,
}

/// Filter to only ROADMAP.md
fn filter_roadmap_only(graph: &Graph) -> Graph {
    let valid_ids: HashSet<&str> = graph.nodes.iter()
        .filter(|n| n.id.starts_with("ROADMAP.md"))
        .map(|n| n.id.as_str())
        .collect();

    Graph {
        nodes: graph.nodes.iter()
            .filter(|n| n.id.starts_with("ROADMAP.md"))
            .cloned()
            .collect(),
        edges: graph.edges.iter()
            .filter(|e| valid_ids.contains(e.source.as_str()) && valid_ids.contains(e.target.as_str()))
            .cloned()
            .collect(),
    }
}

/// Filter to only ARCHIVE.md
fn filter_archive_only(graph: &Graph) -> Graph {
    let valid_ids: HashSet<&str> = graph.nodes.iter()
        .filter(|n| n.id.starts_with("ARCHIVE.md"))
        .map(|n| n.id.as_str())
        .collect();

    Graph {
        nodes: graph.nodes.iter()
            .filter(|n| n.id.starts_with("ARCHIVE.md"))
            .cloned()
            .collect(),
        edges: graph.edges.iter()
            .filter(|e| valid_ids.contains(e.source.as_str()) && valid_ids.contains(e.target.as_str()))
            .cloned()
            .collect(),
    }
}

/// Filter to only roadmap/*.md (detail docs)
fn filter_roadmap_docs_only(graph: &Graph) -> Graph {
    let valid_ids: HashSet<&str> = graph.nodes.iter()
        .filter(|n| n.id.starts_with("roadmap/"))
        .map(|n| n.id.as_str())
        .collect();

    Graph {
        nodes: graph.nodes.iter()
            .filter(|n| n.id.starts_with("roadmap/"))
            .cloned()
            .collect(),
        edges: graph.edges.iter()
            .filter(|e| valid_ids.contains(e.source.as_str()) && valid_ids.contains(e.target.as_str()))
            .cloned()
            .collect(),
    }
}

#[derive(Clone, Serialize)]
struct WsMessage {
    #[serde(rename = "type")]
    msg_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<String>,
}

#[derive(Serialize)]
struct GraphData {
    roadmap: Graph,
    archive: Graph,
    docs: Graph,
}

#[derive(Deserialize)]
struct UpdateTaskRequest {
    status: Option<String>,
}

// Re-export ops types for API use
use crate::ops::AddTaskInput;

#[derive(Deserialize)]
struct UnarchiveRequest {
    task_id: Option<String>,
}

#[derive(Serialize)]
struct ApiResponse {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

pub async fn run(root: PathBuf, port: u16) -> Result<()> {
    // Initial graph
    let graph = scan::run_all(&root)?;
    let graph = Arc::new(tokio::sync::RwLock::new(graph));

    // WebSocket broadcast channel
    let (ws_tx, _) = broadcast::channel::<WsMessage>(16);

    // Start file watcher
    let state = AppState {
        root: root.clone(),
        graph: graph.clone(),
        ws_tx: ws_tx.clone(),
    };
    spawn_file_watcher(root.clone(), graph.clone(), ws_tx.clone());

    // Build router
    let web_dir = root.join("web");
    let app = Router::new()
        // API routes
        .route("/api/graph", get(get_graph))
        .route("/api/status", get(get_status))
        .route("/api/tasks/:id", get(get_task))
        .route("/api/tasks/:id", post(update_task))
        .route("/api/tasks/:id", delete(delete_task))
        .route("/api/tasks", post(add_task))
        .route("/api/archive", post(archive_tasks))
        .route("/api/unarchive", post(unarchive_tasks))
        .route("/api/scan", post(rescan))
        // WebSocket
        .route("/ws", get(ws_handler))
        // Index page
        .route("/", get(index_handler))
        // Static files from project root (for roadmap/*.md, ARCHIVE.md, etc.)
        .fallback_service(ServeDir::new(root.clone()).fallback(ServeDir::new(&web_dir)))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("topo serve running at http://localhost:{}", port);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

fn spawn_file_watcher(
    root: PathBuf,
    graph: Arc<tokio::sync::RwLock<Graph>>,
    ws_tx: broadcast::Sender<WsMessage>,
) {
    tokio::spawn(async move {
        use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};

        let (tx, rx) = std::sync::mpsc::channel();

        let mut watcher = RecommendedWatcher::new(tx, Config::default()).unwrap();
        watcher.watch(&root, RecursiveMode::NonRecursive).unwrap();

        // Also watch ROADMAP.md and ARCHIVE.md specifically
        let roadmap = root.join("ROADMAP.md");
        let archive = root.join("ARCHIVE.md");
        if roadmap.exists() {
            watcher.watch(&roadmap, RecursiveMode::NonRecursive).unwrap();
        }
        if archive.exists() {
            watcher.watch(&archive, RecursiveMode::NonRecursive).unwrap();
        }

        loop {
            match rx.recv() {
                Ok(Ok(event)) => {
                    // Check if it's a markdown file change
                    let is_md = event.paths.iter().any(|p| {
                        p.extension().map(|e| e == "md").unwrap_or(false)
                    });
                    if is_md {
                        // Rescan and broadcast
                        if let Ok(new_graph) = scan::run_all(&root) {
                            {
                                let mut g = graph.write().await;
                                *g = new_graph.clone();
                            }
                            let data = GraphData {
                                roadmap: filter_roadmap_only(&new_graph),
                                archive: filter_archive_only(&new_graph),
                                docs: filter_roadmap_docs_only(&new_graph),
                            };
                            let _ = ws_tx.send(WsMessage {
                                msg_type: "graph_update".into(),
                                data: Some(serde_json::to_value(&data).unwrap()),
                                id: None,
                                status: None,
                            });
                        }
                    }
                }
                Ok(Err(e)) => eprintln!("watch error: {:?}", e),
                Err(_) => break,
            }
        }
    });
}

// API handlers

async fn get_graph(State(state): State<AppState>) -> Json<GraphData> {
    let graph = state.graph.read().await;
    Json(GraphData {
        roadmap: filter_roadmap_only(&graph),
        archive: filter_archive_only(&graph),
        docs: filter_roadmap_docs_only(&graph),
    })
}

async fn get_status(State(state): State<AppState>) -> Json<crate::status::StatusOutput> {
    let graph = state.graph.read().await;
    let filtered = filter_roadmap_only(&graph);
    let status = crate::status::build(&filtered);
    Json(status)
}

async fn get_task(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let graph = state.graph.read().await;
    let node = graph.nodes.iter().find(|n| {
        n.id == id || n.metadata.as_ref().and_then(|m| m.get("stable_id")).and_then(|v| v.as_str()) == Some(&id)
    });

    match node {
        Some(n) => Ok(Json(serde_json::to_value(n).unwrap())),
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn update_task(
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(body): Json<UpdateTaskRequest>,
) -> Json<ApiResponse> {
    if let Some(status) = body.status {
        // Resolve ID
        let graph = state.graph.read().await;
        let canonical = crate::resolve::resolve(&graph, &id);

        match canonical {
            Ok(canonical_id) => {
                // Run update in blocking context
                let root = state.root.clone();
                let status_for_update = status.clone();
                let result = tokio::task::spawn_blocking(move || {
                    crate::update::run(&canonical_id, &format!("status={}", status_for_update), &root)
                }).await;

                match result {
                    Ok(Ok(_)) => {
                        // Broadcast update
                        let _ = state.ws_tx.send(WsMessage {
                            msg_type: "task_updated".into(),
                            data: None,
                            id: Some(id.clone()),
                            status: Some(status.clone()),
                        });
                        Json(ApiResponse { success: true, id: Some(id), error: None })
                    }
                    Ok(Err(e)) => Json(ApiResponse { success: false, id: None, error: Some(e.to_string()) }),
                    Err(e) => Json(ApiResponse { success: false, id: None, error: Some(e.to_string()) }),
                }
            }
            Err(e) => Json(ApiResponse { success: false, id: None, error: Some(e.to_string()) }),
        }
    } else {
        Json(ApiResponse { success: false, id: None, error: Some("no status provided".into()) })
    }
}

async fn add_task(
    State(state): State<AppState>,
    Json(body): Json<AddTaskInput>,
) -> Json<ApiResponse> {
    let root = state.root.clone();
    let input = body.clone();

    let result = tokio::task::spawn_blocking(move || {
        crate::ops::add::run(&input, &root)
    }).await;

    match result {
        Ok(Ok(_)) => {
            let _ = state.ws_tx.send(WsMessage {
                msg_type: "graph_update".into(),
                data: None, // Will trigger client to fetch new graph
                id: None,
                status: None,
            });
            Json(ApiResponse { success: true, id: None, error: None })
        }
        Ok(Err(e)) => Json(ApiResponse { success: false, id: None, error: Some(e.to_string()) }),
        Err(e) => Json(ApiResponse { success: false, id: None, error: Some(e.to_string()) }),
    }
}

async fn delete_task(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Json<ApiResponse> {
    // Resolve ID
    let graph = state.graph.read().await;
    let canonical = crate::resolve::resolve(&graph, &id);
    drop(graph);

    match canonical {
        Ok(canonical_id) => {
            let root = state.root.clone();
            let result = tokio::task::spawn_blocking(move || {
                crate::delete::run(&canonical_id, &root)
            }).await;

            match result {
                Ok(Ok(_)) => {
                    let _ = state.ws_tx.send(WsMessage {
                        msg_type: "task_deleted".into(),
                        data: None,
                        id: Some(id.clone()),
                        status: None,
                    });
                    Json(ApiResponse { success: true, id: Some(id), error: None })
                }
                Ok(Err(e)) => Json(ApiResponse { success: false, id: None, error: Some(e.to_string()) }),
                Err(e) => Json(ApiResponse { success: false, id: None, error: Some(e.to_string()) }),
            }
        }
        Err(e) => Json(ApiResponse { success: false, id: None, error: Some(e.to_string()) }),
    }
}

async fn archive_tasks(State(state): State<AppState>) -> Json<ApiResponse> {
    let root = state.root.clone();
    let result = tokio::task::spawn_blocking(move || {
        crate::archive::run(&root, false)
    }).await;

    match result {
        Ok(Ok(_)) => {
            let _ = state.ws_tx.send(WsMessage {
                msg_type: "graph_update".into(),
                data: None,
                id: None,
                status: None,
            });
            Json(ApiResponse { success: true, id: None, error: None })
        }
        Ok(Err(e)) => Json(ApiResponse { success: false, id: None, error: Some(e.to_string()) }),
        Err(e) => Json(ApiResponse { success: false, id: None, error: Some(e.to_string()) }),
    }
}

async fn unarchive_tasks(
    State(state): State<AppState>,
    Json(body): Json<UnarchiveRequest>,
) -> Json<ApiResponse> {
    let root = state.root.clone();
    let task_id = body.task_id.clone();
    let result = tokio::task::spawn_blocking(move || {
        crate::unarchive::run(&root, task_id.as_deref(), false)
    }).await;

    match result {
        Ok(Ok(_)) => {
            let _ = state.ws_tx.send(WsMessage {
                msg_type: "graph_update".into(),
                data: None,
                id: None,
                status: None,
            });
            Json(ApiResponse { success: true, id: None, error: None })
        }
        Ok(Err(e)) => Json(ApiResponse { success: false, id: None, error: Some(e.to_string()) }),
        Err(e) => Json(ApiResponse { success: false, id: None, error: Some(e.to_string()) }),
    }
}

async fn rescan(State(state): State<AppState>) -> Json<ApiResponse> {
    let root = state.root.clone();
    let result = tokio::task::spawn_blocking(move || {
        scan::run_all(&root)
    }).await;

    match result {
        Ok(Ok(new_graph)) => {
            {
                let mut g = state.graph.write().await;
                *g = new_graph.clone();
            }
            let data = GraphData {
                roadmap: filter_roadmap_only(&new_graph),
                archive: filter_archive_only(&new_graph),
                docs: filter_roadmap_docs_only(&new_graph),
            };
            let _ = state.ws_tx.send(WsMessage {
                msg_type: "graph_update".into(),
                data: Some(serde_json::to_value(&data).unwrap()),
                id: None,
                status: None,
            });
            Json(ApiResponse { success: true, id: None, error: None })
        }
        Ok(Err(e)) => Json(ApiResponse { success: false, id: None, error: Some(e.to_string()) }),
        Err(e) => Json(ApiResponse { success: false, id: None, error: Some(e.to_string()) }),
    }
}

// WebSocket handler

// Index page handler - serve web/index.html
async fn index_handler() -> Response {
    match tokio::fs::read_to_string("web/index.html").await {
        Ok(content) => Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "text/html")
            .body(content.into())
            .unwrap(),
        Err(_) => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body("index.html not found".into())
            .unwrap(),
    }
}

// WebSocket handler

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Response {
    ws.on_upgrade(move |socket| handle_ws(socket, state.ws_tx))
}

async fn handle_ws(mut socket: WebSocket, ws_tx: broadcast::Sender<WsMessage>) {
    let mut rx = ws_tx.subscribe();

    while let Ok(msg) = rx.recv().await {
        let json = serde_json::to_string(&msg).unwrap();
        if socket.send(Message::Text(json)).await.is_err() {
            break;
        }
    }
}