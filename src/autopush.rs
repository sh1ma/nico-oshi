use serde::*;
use websocket::{
    stream::sync::NetworkStream, sync::Client, OwnedMessage, WebSocketError, WebSocketResult,
};

use std::fmt::Debug;

pub struct AutoPushClient {
    client: Client<Box<dyn NetworkStream + Send>>,
    debug: bool,
    uaid: Option<String>,
    channel_id: Option<String>,
}

const AUTOPUSH_ENDPOINT: &str = "wss://push.services.mozilla.com/";

impl AutoPushClient {
    pub fn new(url: &str, uaid: Option<String>, channel_id: Option<String>, debug: bool) -> Self {
        let client = websocket::ClientBuilder::new(url)
            .unwrap()
            .connect(None)
            .unwrap();
        Self {
            client,
            debug,
            uaid,
            channel_id,
        }
    }

    fn send(&mut self, message: impl Sized + Serialize) -> WebSocketResult<()> {
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

    // pub fn post_message<'a, Req: Debug + Serialize, Resp: Debug + Deserialize<'a>>(
    //     &mut self,
    //     message: Req,
    // ) -> Result<Resp, String> {
    //     self.send(message);
    //     let resp = self.recv();
    //     if self.debug {
    //         println!("websocket recv: {:?}", &resp);
    //     }

    //     let resp = match resp {
    //         Ok(OwnedMessage::Text(text)) => text.clone(),
    //         Ok(_) => return Err("unexpected message".to_string()),
    //         Err(e) => return Err(e.to_string()),
    //     };

    //     Ok(serde_json::from_str(&resp).unwrap())
    //     // if let Ok(OwnedMessage::Text(text)) = self.recv() {
    //     //     let resp: Resp = serde_json::from_str(&text).unwrap();
    //     //     if self.debug {
    //     //         println!("websocket recv: {:?}", &resp);
    //     //     }
    //     //     Ok(resp)
    //     // } else {
    //     //     Err("unexpected message".to_string())
    //     // }
    // }

    pub fn post_hello(
        &mut self,
        message: ClientHelloMessage,
    ) -> WebSocketResult<ServerHelloMessage> {
        self.send(message)?;
        let resp = self.recv()?;
        if self.debug {
            println!("websocket recv: {:?}", &resp);
        }
        let resp = match resp {
            OwnedMessage::Text(text) => text.clone(),
            _ => return Err(WebSocketError::NoDataAvailable),
        };
        Ok(serde_json::from_str(&resp).unwrap())
    }

    pub fn post_register(
        &mut self,
        message: ClientRegisterMessage,
    ) -> WebSocketResult<ServerRegisterMessage> {
        self.send(message)?;
        let resp = self.recv()?;
        if self.debug {
            println!("websocket recv: {:?}", &resp);
        }
        let resp = match resp {
            OwnedMessage::Text(text) => text.clone(),
            _ => return Err(WebSocketError::NoDataAvailable),
        };
        Ok(serde_json::from_str(&resp).unwrap())
    }

    pub fn receive_notification(&mut self) -> Result<Notification, AutopushClientError> {
        match self.recv() {
            Ok(OwnedMessage::Text(text)) => {
                let notification: Notification = serde_json::from_str(&text).unwrap();
                if self.debug {
                    println!("websocket recv: {:?}", &notification);
                }
                Ok(notification)
            }
            Ok(OwnedMessage::Ping(ping)) => {
                let pong = OwnedMessage::Pong(ping);
                self.client.send_message(&pong).unwrap();
                if self.debug {}
                Err(AutopushClientError {
                    message: "pong".to_string(),
                    error: None,
                })
            }
            Ok(_) => Err(AutopushClientError {
                message: "unexpected message".to_string(),
                error: None,
            }),

            Err(e) => Err(AutopushClientError {
                message: e.to_string(),
                error: Some(e),
            }),
        }
    }

    pub fn reconnect(&mut self) {
        self.client = websocket::ClientBuilder::new(AUTOPUSH_ENDPOINT)
            .unwrap()
            .connect(None)
            .unwrap();
        let _ = self.send(ClientHelloMessage::new(
            true,
            self.uaid.clone(),
            self.channel_id.clone(),
        ));
        let _ = self.recv();
    }
}

#[derive(Debug, Serialize)]
pub struct ClientHelloMessage {
    #[serde(rename = "messageType")]
    message_type: String,
    use_webpush: Option<bool>,
    uaid: Option<String>,
    #[serde(rename = "channelID")]
    channel_id: Option<String>,
}

impl ClientHelloMessage {
    pub fn new(use_webpush: bool, uaid: Option<String>, channel_id: Option<String>) -> Self {
        Self {
            message_type: "hello".to_string(),
            use_webpush: Some(use_webpush),
            uaid: uaid,
            channel_id: channel_id,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ServerHelloMessage {
    #[serde(rename = "messageType")]
    message_type: String,
    status: u32,
    uaid: String,
    #[serde(rename = "channelID")]
    channel_id: Option<String>,
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
pub struct Notification {
    #[serde(rename = "messageType")]
    message_type: String,
    #[serde(rename = "channelID")]
    channel_id: String,
    data: String,
}

#[derive(Debug)]
pub struct AutopushClientError {
    pub message: String,
    pub error: Option<WebSocketError>,
}
