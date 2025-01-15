use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use futures_util::{SinkExt, StreamExt};
use log::{error, info};
use reqwest::{
    header::{HeaderMap, HeaderName},
    Client, Response,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt::Display,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use uuid::Uuid;

const WEBSOCKET_URLS: [&str; 2] = [
    "wss://proxy2.wynd.network:4650",
    "wss://proxy2.wynd.network:4444",
];
const HEADERS_TO_REPLACE: [&str; 10] = [
    "origin",
    "referer",
    "access-control-request-headers",
    "access-control-request-method",
    "access-control-allow-origin",
    "cookie",
    "date",
    "dnt",
    "trailer",
    "upgrade",
];

#[derive(Debug)]
pub enum NodeType {
    Extension,
    Desktop,
    CommunityExtension,
}

impl NodeType {
    pub fn from_str(s: &str) -> Self {
        match s {
            "1x" => NodeType::Extension,
            "2x" => NodeType::Desktop,
            "1.25x" => NodeType::CommunityExtension,
            _ => panic!("Invalid node type"),
        }
    }
}

impl Display for NodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:?}",
            match self {
                NodeType::Extension => "1x",
                NodeType::Desktop => "2x",
                NodeType::CommunityExtension => "1.25x",
            }
        )
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebSocketMessage {
    pub id: String,
    pub action: String,
    #[serde(default)]
    pub data: HashMap<String, serde_json::Value>,
}

pub struct WebSocketClient {
    user_id: String,
    node_type: NodeType,
    retries: usize,
}

impl WebSocketClient {
    pub fn new(user_id: String, node_type: NodeType) -> Self {
        Self {
            user_id,
            node_type,
            retries: 0,
        }
    }

    async fn perform_http_request(
        params: &HashMap<String, serde_json::Value>,
    ) -> Option<HashMap<String, serde_json::Value>> {
        let client = Client::new();
        let url = params["url"].as_str()?;
        let method = params["method"].as_str()?;

        let mut request = client.request(method.parse().ok()?, url);

        // Handle headers
        if let Some(headers) = params.get("headers") {
            if let Some(headers_obj) = headers.as_object() {
                let mut header_map = HeaderMap::new();
                for (k, v) in headers_obj {
                    if !HEADERS_TO_REPLACE.contains(&k.as_str()) {
                        if let Some(v_str) = v.as_str() {
                            if let Ok(header_name) = HeaderName::from_bytes(k.as_bytes()) {
                                header_map.insert(header_name, v_str.parse().unwrap());
                            }
                        }
                    }
                }
                request = request.headers(header_map);
            }
        }

        // Handle body
        if let Some(body) = params.get("body") {
            if let Some(body_str) = body.as_str() {
                if let Ok(decoded) = BASE64.decode(body_str) {
                    request = request.body(decoded);
                }
            }
        }

        match request.send().await {
            Ok(response) => {
                let size = response.content_length().unwrap_or(0);
                info!("Response size: {} bytes", size);
                Some(Self::process_response(response).await)
            }
            Err(e) => {
                error!("Error in HTTP request: {}", e);
                None
            }
        }
    }

    async fn send_ping(&mut self, ws_stream: &mut WebSocketStream<MaybeTlsStream<TcpStream>>) {
        let msg = serde_json::json!({
            "id": Uuid::new_v4().to_string(),
            "action": "PING".to_string(),
            "version": match self.node_type {
                    NodeType::Desktop => "4.30.0".to_string(),
                    _ => "4.26.2".to_string(),
                },
            "data": HashMap::<String, serde_json::Value>::new()
        });
        ws_stream
            .send(Message::Text(msg.to_string().into()))
            .await
            .unwrap();
        info!("Sent ping: {}", msg.to_string());
    }

    async fn process_response(response: Response) -> HashMap<String, serde_json::Value> {
        let mut result = HashMap::new();

        result.insert(
            "url".to_string(),
            serde_json::Value::String(response.url().to_string()),
        );
        result.insert(
            "status".to_string(),
            serde_json::Value::Number(response.status().as_u16().into()),
        );
        result.insert(
            "status_text".to_string(),
            serde_json::Value::String(response.status().to_string()),
        );

        // Process headers
        let headers: HashMap<String, String> = response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        result.insert(
            "headers".to_string(),
            serde_json::to_value(headers).unwrap(),
        );

        // Process body
        let body_bytes = response.bytes().await.unwrap_or_default();
        let body_base64 = BASE64.encode(&body_bytes);
        result.insert("body".to_string(), serde_json::Value::String(body_base64));

        result
    }

    pub async fn start(&mut self) {
        info!("Starting WebSocket client...");
        info!("Node type: {}", self.node_type);

        loop {
            match self.connect().await {
                Ok(mut ws_stream) => {
                    if let Err(e) = self.authenticate(&mut ws_stream).await {
                        error!("Authentication error: {}", e);
                        continue;
                    }

                    info!("Waiting for messages...");

                    let mut ping_interval =
                        tokio::time::interval(tokio::time::Duration::from_secs(60));

                    loop {
                        tokio::select! {
                            _ = ping_interval.tick() => {
                                self.send_ping(&mut ws_stream).await;
                            }
                            msg = ws_stream.next() => {
                                match msg {
                                    Some(Ok(msg)) => self.handle_message(&mut ws_stream, msg).await,
                                    Some(Err(e)) => {
                                        error!("Error receiving message: {}", e);
                                        break;
                                    }
                                    None => break,
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("WebSocket connection error: {}", e);
                    self.retries += 1;
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        }
    }

    async fn send_response(
        &mut self,
        ws_stream: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
        msg_id: String,
        origin_action: String,
        result: HashMap<String, serde_json::Value>,
    ) {
        let response = serde_json::json!({
            "id": msg_id,
            "origin_action": origin_action,
            "result": result
        });
        ws_stream
            .send(Message::Text(response.to_string().into()))
            .await
            .unwrap();

        info!("Sent response: {}", response.to_string());
    }

    async fn handle_message(
        &mut self,
        ws_stream: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
        msg: Message,
    ) {
        if let Message::Text(text) = msg {
            info!("Received message: {}", &text.to_string());
            let message: WebSocketMessage = serde_json::from_str(&text.to_string()).unwrap();
            match message.action.as_str() {
                "HTTP_REQUEST" => {
                    let result = Self::perform_http_request(&message.data).await;
                    if let Some(result) = result {
                        self.send_response(ws_stream, message.id, message.action, result)
                            .await;
                    }
                }
                "PONG" => {
                    self.send_response(ws_stream, message.id, message.action, HashMap::new())
                        .await;
                }
                _ => {
                    // Do nothing
                }
            }
        }
    }

    async fn connect(
        &self,
    ) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>, Box<dyn std::error::Error>> {
        let websocket_url = WEBSOCKET_URLS[self.retries % WEBSOCKET_URLS.len()];
        info!("Connecting to {}", websocket_url);
        let (ws_stream, _) = connect_async(websocket_url).await?;
        info!("Connected to {}", websocket_url);
        Ok(ws_stream)
    }

    async fn authenticate(
        &self,
        ws_stream: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        info!("Authenticating...");
        let auth_message = serde_json::json!({
            "id": Uuid::new_v4().to_string(),
            "origin_action": "AUTH".to_string(),
            "result": {
                "browser_id": Uuid::new_v4().to_string(),
                "user_id": self.user_id.clone(),
                "user_agent": "Mozilla/5.0".to_string(), // You might want to use a proper user-agent generator
                "timestamp": SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
                "device_type": match self.node_type {
                    NodeType::Desktop => "desktop".to_string(),
                    _ => "extension".to_string(),
                },
                "version": match self.node_type {
                    NodeType::Desktop => "4.30.0".to_string(),
                    _ => "4.26.2".to_string(),
                },
                "extension_id": match self.node_type {
                    NodeType::Desktop => None,
                    NodeType::Extension => Some("ilehaonighjijnmpnagapkhpcdbhclfg".to_string()),
                    NodeType::CommunityExtension => Some("lkbnfiajjmbhnfledhphioinpickokdi".to_string()),
                    
                },
            }
        });

        ws_stream
            .send(Message::Text(serde_json::to_string(&auth_message)?.into()))
            .await?;
        Ok(())
    }
}
