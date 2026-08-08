use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use anyhow::Result;
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Router,
};
use tokio::net::TcpListener;
use tokio::runtime;
use tokio::sync::oneshot;
use tokio::sync::mpsc;

const RESPONSE_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug)]
pub enum Response {
    Success,
    Error(String),
    Timeout,
}

impl Response {
    fn err(s: impl ToString) -> Self {
        Response::Error(s.to_string())
    }
}

impl IntoResponse for Response {
    fn into_response(self) -> axum::response::Response {
        match self {
            Response::Success => StatusCode::OK.into_response(),
            Response::Error(err) => (StatusCode::INTERNAL_SERVER_ERROR, err).into_response(),
            Response::Timeout => StatusCode::SERVICE_UNAVAILABLE.into_response(),
        }
    }
}

#[derive(Debug)]
pub enum Command {
    Pause,
    Resume,
}

impl Command {
    pub fn describe(&self) -> String {
        match self {
            Command::Pause => "pause".to_string(),
            Command::Resume => "resume".to_string(),
        }
    }
}

#[derive(Debug)]
pub struct Request {
    pub response_sender: oneshot::Sender<Response>,
    pub command: Command,
}

#[derive(Debug)]
pub struct Api {
    thread_handle: JoinHandle<()>,
    command_queue: mpsc::Receiver<Request>,
}

macro_rules! simple_command {
    ($name:ident, $command:expr) => {
        async fn $name(State(queue): State<Arc<mpsc::Sender<Request>>>) -> Response {
            Self::send_command(queue, $command).await
        }
    };
}

impl Api {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(10);
        let thread_handle = std::thread::spawn(move || {
            let rt = runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("tokio runtime should be created");
            rt.block_on(Self::serve(tx)).expect("API server must succeed");
        });
        Self {
            thread_handle,
            command_queue: rx,
        }
    }

    pub fn try_recv(&mut self) -> Result<Request, mpsc::error::TryRecvError> {
        self.command_queue.try_recv()
    }

    pub fn shutdown(&mut self) {
        self.command_queue.close();
        // this doesn't handle draining the queue and/or telling the server thread to end because we
        // can't safely synchronize with other threads from DllMain
    }

    async fn serve(command_queue: mpsc::Sender<Request>) -> Result<()> {
        let app = Router::new()
            .route("/pause", post(Self::pause))
            .route("/resume", post(Self::resume))
            .with_state(Arc::new(command_queue));

        let listener = TcpListener::bind("127.0.0.1:0").await?;
        log::info!("API server listening on port {}", listener.local_addr()?.port());

        axum::serve(listener, app).await?;
        Ok(())
    }

    async fn send_command(command_queue: Arc<mpsc::Sender<Request>>, command: Command) -> Response {
        let (response_sender, response_receiver) = oneshot::channel();
        command_queue.send(Request {
            response_sender,
            command,
        }).await.unwrap(); // unwrap because we know the receiver is still open
        match tokio::time::timeout(RESPONSE_TIMEOUT, response_receiver).await {
            Ok(Ok(response)) => response,
            Ok(Err(_)) => Response::err("Failed to receive response"),
            Err(_) => Response::Timeout,
        }
    }

    simple_command!(pause, Command::Pause);
    simple_command!(resume, Command::Resume);
}