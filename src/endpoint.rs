use base64::engine::{general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde::*;

pub struct NicoPushEndpointClient {
    pub url: String,
    pub session: String,
    http_client: reqwest::Client,
}

impl NicoPushEndpointClient {
    pub fn new(url: String, session: String) -> Self {
        Self {
            url,
            session,
            http_client: reqwest::Client::new(),
        }
    }

    pub async fn register(&self, push_endpoint: String, auth: Vec<u8>, p256dh: Vec<u8>) {
        let body = RegisterWebPushEndpointRequest::new(push_endpoint, auth, p256dh);

        let req = self
            .http_client
            .post(self.url.clone())
            .header("Cookie", self.session.clone())
            .header("X-Frontend-Id", 8)
            .header("X-Request-With", "https://account.nicovideo.jp/my/account")
            .json(&body);

        let resp = req.send().await.unwrap();
        if resp.status().is_success() {
            println!("register success");
        } else {
            println!("failed");
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct RegisterWebPushEndpointRequest {
    #[serde(rename = "destApp")]
    dest_app: String,
    endpoint: RegisterWebPushEndpointRequestEndpoint,
}

impl RegisterWebPushEndpointRequest {
    fn new(endpoint: String, auth: Vec<u8>, p256dh: Vec<u8>) -> Self {
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
struct RegisterWebPushEndpointRequestEndpoint {
    endpoint: String,
    auth: String,
    p256dh: String,
}
