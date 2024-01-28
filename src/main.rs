use elliptic_curve::rand_core::OsRng;

use uuid::Uuid;

use autopush::AutoPushClient;
use endpoint::NicoPushEndpointClient;

use crate::autopush::{
    ClientHelloMessage, ClientRegisterMessage, ServerHelloMessage, ServerRegisterMessage,
};

mod autopush;
mod endpoint;

const AUTOPUSH_ENDPOINT: &str = "wss://push.services.mozilla.com/";
const NICONICO_WEBPUSH_ENDPOINT: &str =
    "https://api.push.nicovideo.jp/v1/nicopush/webpush/endpoints.json";

#[tokio::main]
async fn main() {
    // let niconico_user_session = std::env::var("NICONICO_USER_SESSION").unwrap();
    // let niconico_user_session_secure = std::env::var("NICONICO_USER_SESSION_SECURE").unwrap();
    let raw_cookie = std::env::var("RAW_COOKIE").unwrap();
    let niconico_application_server_key = std::env::var("NICONICO_APPLICATION_SERVER_KEY").unwrap();

    let mut autopush_client = AutoPushClient::new(AUTOPUSH_ENDPOINT, None, None, true);
    let endpoint_client =
        NicoPushEndpointClient::new(NICONICO_WEBPUSH_ENDPOINT.to_string(), raw_cookie);

    let _: ServerHelloMessage = autopush_client
        .post_hello(ClientHelloMessage::new(true, None, None))
        .unwrap();

    let channel_id = Uuid::new_v4().to_string();
    let register_resp: ServerRegisterMessage = autopush_client
        .post_register(ClientRegisterMessage::new(
            channel_id,
            niconico_application_server_key,
        ))
        .unwrap();

    let push_endpoint = register_resp.push_endpoint;

    println!("{}", push_endpoint);

    let auth: Vec<u8> = (0..15).map(|_| rand::random::<u8>()).collect();
    let secret_key = p256::ecdh::EphemeralSecret::random(&mut OsRng);
    let public_key = secret_key.public_key();
    let public_key_string = public_key.to_sec1_bytes().to_vec();

    endpoint_client
        .register(
            push_endpoint.clone(),
            auth.clone(),
            public_key_string.clone(),
        )
        .await;

    loop {
        match autopush_client.receive_notification() {
            Ok(notification) => {
                println!("notification: {:?}", notification);
            }
            Err(e) => match e.error {
                Some(err) => match err {
                    websocket::WebSocketError::NoDataAvailable => {
                        println!("コネクション切断。再接続を試みます。");
                        autopush_client.reconnect();
                    }
                    _ => {
                        println!("websocket error: {:?}", err);
                    }
                },
                None => {
                    println!("unexpected: {:?}", e.message);
                }
            },
        }
    }

    // loop {
    //     let msg = wsconn.recv_message();
    //     match msg {
    //         Ok(msg) => {
    //             if msg.is_ping() {
    //                 wsconn.send_message(&Message::pong(vec![])).unwrap();
    //                 println!("pong");
    //                 continue;
    //             }
    //             println!("{:?}", msg);
    //             if let OwnedMessage::Text(msg) = msg {
    //                 let notification = serde_json::from_str::<Notification>(&msg).unwrap();
    //                 println!("{:?}", notification);
    //                 let data = URL_SAFE_NO_PAD.decode(notification.data).unwrap();
    //                 let mut cursor = Cursor::new(data);
    //                 let mut salt = [0u8; 16];
    //                 cursor.read_exact(&mut salt).unwrap();
    //                 let mut rs = [0u8; 4];
    //                 cursor.read_exact(&mut rs).unwrap();

    //                 let mut idlen_byte = [0u8; 1];
    //                 cursor.read_exact(&mut idlen_byte).unwrap();
    //                 let idlen = idlen_byte[0];
    //                 println!("{}", idlen);
    //                 let mut key = vec![0u8; idlen as usize];
    //                 cursor.read_exact(&mut key).unwrap();
    //                 let mut cipher_text = vec![];
    //                 cursor.read_to_end(&mut cipher_text).unwrap();
    //                 let shared_secret =
    //                     secret_key.diffie_hellman(&p256::PublicKey::from_sec1_bytes(&key).unwrap());
    //                 let mut auth_info: Vec<u8> = vec![];
    //                 auth_info.append("WebPush: info\0".as_bytes().to_vec().as_mut());
    //                 auth_info.append(public_key.to_sec1_bytes().to_vec().as_mut());
    //                 auth_info.append(&mut key.clone());
    //                 let auth_kdf = hkdf::Hkdf::<sha2::Sha256>::new(
    //                     Some(&auth),
    //                     shared_secret.raw_secret_bytes(),
    //                 );
    //                 let mut prk = [0u8; 32];
    //                 auth_kdf.expand(&[], &mut prk).unwrap();

    //                 let mut key_info: Vec<u8> = vec![];
    //                 key_info.append("Content-Encoding: aes128gcm\0".as_bytes().to_vec().as_mut());
    //                 let prk_kdf = hkdf::Hkdf::<sha2::Sha256>::new(Some(&salt), &prk);
    //                 let mut cek = [0u8; 16];
    //                 prk_kdf.expand(&key_info, &mut cek).unwrap();

    //                 let mut nonce_info: Vec<u8> = vec![];
    //                 nonce_info.append("Content-Encoding: nonce\0".as_bytes().to_vec().as_mut());
    //                 let kdf = hkdf::Hkdf::<sha2::Sha256>::new(Some(&salt), &prk);
    //                 let mut nonce = [0u8; 12];
    //                 kdf.expand(&nonce_info, &mut nonce).unwrap();
    //                 let cek_key = aes_gcm::Key::<Aes128Gcm>::from_slice(&cek);
    //                 let nonce_key = aes_gcm::Nonce::from_slice(&nonce);
    //                 let mut gcm = aes_gcm::Aes128Gcm::new(&cek_key);
    //                 let plaintext = gcm.decrypt(nonce_key, cipher_text.as_slice());
    //                 match plaintext {
    //                     Ok(plaintext) => {
    //                         println!("{}", String::from_utf8(plaintext).unwrap());
    //                     }
    //                     Err(e) => {
    //                         println!("{:?}", e);
    //                     }
    //                 }
    //             }
    //         }
    //         Err(e) => match e {
    //             websocket::WebSocketError::NoDataAvailable => {
    //                 println!("nodata");
    //                 break;
    //             }
    //             _ => {
    //                 println!("{:?}", e);
    //             }
    //         },
    //     }
    // }
}
