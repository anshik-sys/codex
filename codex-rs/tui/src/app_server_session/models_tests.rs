use super::*;
use crate::app_server_session::ThreadParamsMode;
use futures::SinkExt;
use futures::StreamExt;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;
use tokio_tungstenite::tungstenite::Message;

/// Sends one `fetch_models` call through a recording server and returns its `modelProvider`.
async fn requested_model_provider(model_provider_override: Option<&str>) -> Value {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let websocket_url = format!("ws://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut socket = tokio_tungstenite::accept_async(stream).await.unwrap();
        while let Some(Ok(Message::Text(text))) = socket.next().await {
            let request: Value = serde_json::from_str(&text).unwrap();
            if request["id"].is_null() {
                continue;
            }
            let initialize = request["method"] == "initialize";
            let mut response = if initialize {
                json!({"result": {"userAgent": "models-test", "platformFamily": "unix", "platformOs": "linux"}})
            } else {
                json!({"result": {"data": [], "nextCursor": null}})
            };
            response["id"] = request["id"].clone();
            socket
                .send(Message::Text(response.to_string().into()))
                .await
                .unwrap();
            if !initialize {
                return request;
            }
        }
        panic!("no model/list request arrived");
    });
    let client = crate::connect_remote_app_server(crate::RemoteAppServerEndpoint::WebSocket {
        websocket_url,
        auth_token: None,
    })
    .await
    .unwrap();
    let mut session = AppServerSession::new(client, ThreadParamsMode::Remote);
    session.model_provider_override = model_provider_override.map(str::to_string);
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    session.fetch_models(Uuid::new_v4(), AppEventSender::new(tx));

    assert!(matches!(
        rx.recv().await,
        Some(AppEvent::ModelsLoaded { result: Ok(_), .. })
    ));
    let request = server.await.unwrap();
    assert_eq!(request["method"], "model/list");
    request["params"]["modelProvider"].clone()
}

#[tokio::test]
async fn model_picker_lists_the_switched_provider() {
    assert_eq!(
        requested_model_provider(Some("openrouter")).await,
        json!("openrouter")
    );
    assert_eq!(requested_model_provider(None).await, Value::Null);
}
