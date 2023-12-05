use std::borrow::Cow;
use base64::{engine::general_purpose, Engine};
use chrono::{DateTime, Local, Utc};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use reqwest::Request;
use serde::{Deserialize, Serialize};

#[derive(Default)]
pub struct Builder {
    api_url: Cow<'static, str>,
    base_url: Cow<'static, str>,
}

impl Builder {
    pub fn api_url(mut self, api_url: impl Into<Cow<'static, str>>) -> Self {
        self.api_url = api_url.into();
        self
    }

    pub fn base_url(mut self, base_url: impl Into<Cow<'static, str>>) -> Self {
        self.base_url = base_url.into();
        self
    }

    pub fn build(self) -> Client {
        Client { builder: self }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UserCredentials<'a> {
    user_id: &'a str,
    password: &'a str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LoginData<'a> {
    device_key: &'a str,
    device_type: usize,
    user_credential: UserCredentials<'a>,
}

pub struct Client {
    builder: Builder,
}

impl Client {
    pub fn builder() -> Builder {
        Builder::default()
    }

    pub fn us() -> Self {
        Self::builder()
            .base_url("api.owners.kia.com")
            .api_url("apigw/v1")
            .build()
    }

    pub async fn login(&self, username: &str,password: &str) -> String {
        let url = format!(
            "https://{}/{}/prof/authUser",
            &self.builder.base_url, &self.builder.api_url
        );
        let client = reqwest::Client::new();
        let body = LoginData {
            device_key: "",
            device_type: 2,
            user_credential: UserCredentials {
                user_id: username,
                password,
            },
        };

        let local_time = Local::now();
        let offset = local_time.offset().local_minus_utc() / 3600;

        let utc_time: DateTime<Utc> = Utc::now();
        let formatted_date = utc_time.format("%a, %d %b %Y %H:%M:%S GMT").to_string();

        fn generate_device_id() -> String {
            let random_chars: String = thread_rng()
                .sample_iter(&Alphanumeric)
                .take(22)
                .map(char::from)
                .collect();

            let token = general_purpose::URL_SAFE_NO_PAD.encode(&random_chars);

            format!("{}:{}", random_chars, token)
        }

        let res = client
            .post(url)
            .header("content-type", "application/json;charset=UTF-8")
            .header("accept", "application/json, text/plain, */*")
            .header("accept-encoding", "gzip, deflate, br")
            .header("accept-language", "en-US,en;q=0.9")
            .header("apptype", "L")
            .header("appversion", "4.10.0")
            .header("clientid", "MWAMOBILE")
            .header("from", "SPA")
            .header("host", &*self.builder.base_url)
            .header("language", "0")
            .header("offset", offset.to_string())
            .header("ostype", "Android")
            .header("osversion", "11")
            .header("secretkey", "98er-w34rf-ibf3-3f6h")
            .header("to", "APIGW")
            .header("tokentype", "G")
            .header("user-agent", "okhttp/3.12.1")
            .header("date", formatted_date)
            .header("deviceid", generate_device_id())
            .json(&body)
            .send()
            .await
            .unwrap();

        res.headers().get("sid").unwrap().to_str().unwrap().to_owned()
    }
}