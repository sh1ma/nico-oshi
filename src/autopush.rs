use serde::*;
use websocket::{
    stream::sync::NetworkStream, sync::Client, OwnedMessage, WebSocketError, WebSocketResult,
};

use std::fmt::Debug;

pub struct AutoPushClient {
    client: Client<Box<dyn NetworkStream + Send>>,
    debug: bool,
    uaid: Option<String>,
}

const AUTOPUSH_ENDPOINT: &str = "wss://push.services.mozilla.com/";

impl AutoPushClient {
    pub fn new(url: &str, debug: bool) -> Self {
        let client = websocket::ClientBuilder::new(url)
            .unwrap()
            .connect(None)
            .unwrap();
        Self { client, debug }
    }

    fn send(&self, message: impl Sized + Serialize) -> WebSocketResult<()> {
        let request_payload = serde_json::to_string(&message).unwrap();
        let message_text = websocket::Message::text(&request_payload);
        if self.debug {
            println!("websocket send: {:?}", &message_text);
        }
        self.client.send_message(&message_text)
    }

    fn recv(&mut self) -> Result<OwnedMessage, websocket::WebSocketError> {
        self.client.recv_message()
    }

    pub fn post_message<Req: Debug + Serialize, Resp: Debug + Deserialize<'static>>(
        &self,
        message: Req,
    ) -> Result<Resp, String> {
        self.send(message);
        if let Ok(OwnedMessage::Text(text)) = self.recv() {
            let resp: Resp = serde_json::from_str(&text).unwrap();
            if self.debug {
                println!("websocket recv: {:?}", &resp);
            }
            Ok(resp)
        } else {
            Err("unexpected message".to_string())
        }
    }

    pub fn receive_notification(&self) -> Result<Notification, AutopushClientError> {
        match self
        // if let Ok(OwnedMessage::Text(text)) = self.recv() {
        //     let notification: Notification = serde_json::from_str(&text).unwrap();
        //     if self.debug {
        //         println!("websocket recv: {:?}", &notification);
        //     }
        //     notification
        // } else if
    }
}

#[derive(Debug, Serialize)]
pub struct ClientHelloMessage {
    #[serde(rename = "messageType")]
    message_type: String,
    use_webpush: Option<bool>,
    uaid: Option<String>,
}

impl ClientHelloMessage {
    pub fn new(use_webpush: bool, uaid: Option<String>) -> Self {
        Self {
            message_type: "hello".to_string(),
            use_webpush: Some(use_webpush),
            uaid: uaid,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ServerHelloMessage {
    #[serde(rename = "messageType")]
    message_type: String,
    status: String,
    uaid: String,
}

#[derive(Debug, Serialize)]
pub struct ClientRegisterMessage {
    #[serde(rename = "messageType")]
    message_type: String,
    #[serde(rename = "channelID")]
    channel_id: String,
    key: String,
}

impl ClientRegisterMessage {
    pub fn new(channel_id: String, key: String) -> Self {
        Self {
            message_type: "register".to_string(),
            channel_id,
            key,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ServerRegisterMessage {
    #[serde(rename = "messageType")]
    message_type: String,
    #[serde(rename = "channelID")]
    channel_id: String,
    status: u32,
    #[serde(rename = "pushEndpoint")]
    pub push_endpoint: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Notification {
    #[serde(rename = "messageType")]
    message_type: String,
    #[serde(rename = "channelID")]
    channel_id: String,
    data: String,
}

#[derive(Debug)]
pub struct AutopushClientError {
    message: String,
}
