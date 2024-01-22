use std::{
    collections::HashMap,
    fmt::format,
    hash::Hash,
    io::{Cursor, Read},
    net::TcpStream,
};

use aes_gcm::{aead::AeadMut, Aes128Gcm, KeyInit};
use elliptic_curve::{
    bigint::Random,
    ecdh::{self, EphemeralSecret},
    pkcs8::der::{pem::Base64Encoder, Encode},
    rand_core::{le, OsRng},
    CurveArithmetic, PrimeCurveArithmetic,
};

use base64::engine::{
    general_purpose::{URL_SAFE, URL_SAFE_NO_PAD},
    Engine as _,
};
use h2::client;

use hkdf::hmac::Hmac;
use hyper_util::rt::TokioIo;
use reqwest::{
    header::{HeaderMap, HeaderValue, USER_AGENT},
    Version,
};
use serde::*;
use serde_json::json;
use uuid::Uuid;
use websocket::{
    header::{ContentType, Cookie},
    Message, OwnedMessage,
};

const AUTOPUSH_ENDPOINT: &str = "wss://push.services.mozilla.com/";
const NICONICO_WEBPUSH_ENDPOINT: &str =
    "https://api.push.nicovideo.jp/v1/nicopush/webpush/endpoints.json";

#[tokio::main]
async fn main() {
    let niconico_user_session = std::env::var("NICONICO_USER_SESSION").unwrap();
    let niconico_user_session_secure = std::env::var("NICONICO_USER_SESSION_SECURE").unwrap();

    let raw_cookie = std::env::var("RAW_COOKIE").unwrap();

    println!("{}", AUTOPUSH_ENDPOINT);
    let mut wsconn = websocket::ClientBuilder::new(AUTOPUSH_ENDPOINT)
        .unwrap()
        .connect(None)
        .unwrap();

    let rs = serde_json::to_string(&ClientHelloMessage {
        message_type: "hello".to_string(),
        use_webpush: Some(true),
        // uaid: UAID.to_string(),
    })
    .unwrap();
    println!("{}", rs);

    let msg = Message::text(rs);

    wsconn.send_message(&msg).unwrap();
    let r = wsconn.recv_message();
    println!("{:?}", r);

    let channel_id = Uuid::new_v4().to_string();

    let register_message = serde_json::to_string(&ClientRegisterMessage {
        message_type: "register".to_string(),
        channel_id: channel_id.clone(),
    })
    .unwrap();

    println!("{}", register_message);

    let msg = Message::text(register_message);

    wsconn.send_message(&msg).unwrap();
    let push_endpoint = {
        if let OwnedMessage::Text(register_resp) = wsconn.recv_message().unwrap() {
            println!("{:?}", register_resp);
            let resp_model = serde_json::from_str::<ServerRegisterMessage>(&register_resp).unwrap();
            println!("{:?}", register_resp);
            resp_model.push_endpoint
        } else {
            panic!("register response is not text");
        }
    };

    println!("{}", push_endpoint);

    let secret_key = p256::ecdh::EphemeralSecret::random(&mut OsRng);
    let public_key = secret_key.public_key();
    let public_key_string = public_key.to_sec1_bytes().to_vec();
    let auth: Vec<u8> = (0..15).map(|_| rand::random::<u8>()).collect();

    let register_webpush_endpoint_request = RegisterWebPushEndpointRequest::new(
        auth.clone(),
        push_endpoint.clone(),
        public_key_string.clone(),
    );

    println!("{:?}", register_webpush_endpoint_request);

    // let client = reqwest::blocking::Client::new();

    let mut headers: HeaderMap = HeaderMap::new();
    headers.insert("X-Frontend-Id", HeaderValue::from_static("9"));
    headers.insert(
        "X-Request-With",
        // "https://account.nicovideo.jp/my/account".parse().unwrap(),
        HeaderValue::from_static("https://account.nicovideo.jp/my/account"),
    );
    headers.insert("Accept", HeaderValue::from_static("application/json"));
    headers.insert("Cookie", HeaderValue::from_str(&raw_cookie).unwrap());
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static(
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:121.0) Gecko/20100101 Firefox/121.0",
        ),
    );
    headers.insert("Content-Type", HeaderValue::from_static("application/json"));

    println!(
        "{}",
        serde_json::to_string(&register_webpush_endpoint_request).unwrap()
    );

    let client = reqwest::Client::builder()
        .default_headers(headers.clone())
        .build()
        .unwrap();

    let req = client
        .post(NICONICO_WEBPUSH_ENDPOINT)
        // .headers(headers)
        .json(&register_webpush_endpoint_request);
    println!("{:?}", req);
    let resp = req.send().await;

    loop {
        let msg = wsconn.recv_message();
        match msg {
            Ok(msg) => {
                if msg.is_ping() {
                    wsconn.send_message(&Message::pong(vec![])).unwrap();
                    println!("pong");
                    continue;
                }
                println!("{:?}", msg);
                if let OwnedMessage::Text(msg) = msg {
                    let notification = serde_json::from_str::<Notification>(&msg).unwrap();
                    println!("{:?}", notification);
                    let data = URL_SAFE_NO_PAD.decode(notification.data).unwrap();
                    let mut cursor = Cursor::new(data);
                    let mut salt = [0u8; 16];
                    cursor.read_exact(&mut salt).unwrap();
                    let mut rs = [0u8; 4];
                    cursor.read_exact(&mut rs).unwrap();

                    let mut idlen_byte = [0u8; 1];
                    cursor.read_exact(&mut idlen_byte).unwrap();
                    let idlen = idlen_byte[0];
                    println!("{}", idlen);
                    let mut key = vec![0u8; idlen as usize];
                    cursor.read_exact(&mut key).unwrap();
                    let mut cipher_text = vec![];
                    cursor.read_to_end(&mut cipher_text).unwrap();
                    let shared_secret =
                        secret_key.diffie_hellman(&p256::PublicKey::from_sec1_bytes(&key).unwrap());
                    let mut auth_info: Vec<u8> = vec![];
                    auth_info.append("WebPush: info\0".as_bytes().to_vec().as_mut());
                    auth_info.append(public_key.to_sec1_bytes().to_vec().as_mut());
                    auth_info.append(&mut key.clone());
                    let auth_kdf = hkdf::Hkdf::<sha2::Sha256>::new(
                        Some(&auth),
                        shared_secret.raw_secret_bytes(),
                    );
                    let mut prk = [0u8; 32];
                    auth_kdf.expand(&[], &mut prk).unwrap();

                    let mut key_info: Vec<u8> = vec![];
                    key_info.append("Content-Encoding: aes128gcm\0".as_bytes().to_vec().as_mut());
                    let prk_kdf = hkdf::Hkdf::<sha2::Sha256>::new(Some(&salt), &prk);
                    let mut cek = [0u8; 16];
                    prk_kdf.expand(&key_info, &mut cek).unwrap();

                    let mut nonce_info: Vec<u8> = vec![];
                    nonce_info.append("Content-Encoding: nonce\0".as_bytes().to_vec().as_mut());
                    let kdf = hkdf::Hkdf::<sha2::Sha256>::new(Some(&salt), &prk);
                    let mut nonce = [0u8; 12];
                    kdf.expand(&nonce_info, &mut nonce).unwrap();
                    let cek_key = aes_gcm::Key::<Aes128Gcm>::from_slice(&cek);
                    let nonce_key = aes_gcm::Nonce::from_slice(&nonce);
                    let mut gcm = aes_gcm::Aes128Gcm::new(&cek_key);
                    let plaintext = gcm.decrypt(nonce_key, cipher_text.as_slice());
                    match plaintext {
                        Ok(plaintext) => {
                            println!("{}", String::from_utf8(plaintext).unwrap());
                        }
                        Err(e) => {
                            println!("{:?}", e);
                        }
                    }
                }
            }
            Err(e) => match e {
                websocket::WebSocketError::NoDataAvailable => {
                    println!("nodata");
                    break;
                }
                _ => {
                    println!("{:?}", e);
                }
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct ClientHelloMessage {
    #[serde(rename = "messageType")]
    message_type: String,
    use_webpush: Option<bool>,
    // uaid: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ClientRegisterMessage {
    #[serde(rename = "messageType")]
    message_type: String,
    #[serde(rename = "channelID")]
    channel_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ServerRegisterMessage {
    #[serde(rename = "messageType")]
    message_type: String,
    #[serde(rename = "channelID")]
    channel_id: String,
    status: u32,
    #[serde(rename = "pushEndpoint")]
    push_endpoint: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct RegisterWebPushEndpointRequest {
    #[serde(rename = "destApp")]
    dest_app: String,
    endpoint: RegisterWebPushEndpointRequestEndpoint,
}

#[derive(Debug, Serialize, Deserialize)]
struct RegisterWebPushEndpointRequestEndpoint {
    endpoint: String,
    auth: String,
    p256dh: String,
}

impl RegisterWebPushEndpointRequest {
    fn new(auth: Vec<u8>, endpoint: String, p256dh: Vec<u8>) -> Self {
        println!("{:?}", p256dh);
        let auth = URL_SAFE_NO_PAD.encode(&auth);
        let p256dh = URL_SAFE_NO_PAD.encode(&p256dh);

        Self {
            dest_app: "nico_account_webpush".to_string(),
            endpoint: RegisterWebPushEndpointRequestEndpoint {
                endpoint,
                auth,
                p256dh,
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Notification {
    #[serde(rename = "messageType")]
    message_type: String,
    #[serde(rename = "channelID")]
    channel_id: String,
    data: String,
}
