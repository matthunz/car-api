use base64::{engine::general_purpose, Engine};
use chrono::{DateTime, Local, Utc};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use reqwest::RequestBuilder;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, time::Duration};

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

    pub async fn login(&self, username: &str, password: &str) -> String {
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

        res.headers()
            .get("sid")
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned()
    }

    pub async fn vehicles(&self, token: &str) -> Vec<Vehicle> {
        let url = format!(
            "https://{}/{}/ownr/gvl",
            &self.builder.base_url, &self.builder.api_url
        );
        let client = reqwest::Client::new();
        let res = self
            .headers(client.get(url))
            .header("sid", token)
            .send()
            .await
            .unwrap();

        let json: Vehicles = res.json().await.unwrap();
        json.payload.vehicle_summary
    }

    pub async fn lock(&self, session_key: &str, vehicle_key: &str) {
        let action_key = self.start_lock(session_key, vehicle_key).await;

        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;
            let status = self
                .check_status(session_key, vehicle_key, &action_key)
                .await;
            if status.remote_status == 0 {
                break;
            }
        }
    }

    pub async fn unlock(&self, session_key: &str, vehicle_key: &str) {
        let action_key = self.start_unlock(session_key, vehicle_key).await;

        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;
            let status = self
                .check_status(session_key, vehicle_key, &action_key)
                .await;
            if status.remote_status == 0 {
                break;
            }
        }
    }

    pub async fn start_lock(&self, session_key: &str, vehicle_key: &str) -> String {
        let client = reqwest::Client::new();
        let url = format!(
            "https://{}/{}/rems/door/lock",
            &self.builder.base_url, &self.builder.api_url
        );
        let res = self
            .headers(client.get(url))
            .header("sid", session_key)
            .header("vinkey", vehicle_key)
            .send()
            .await
            .unwrap();

        res.headers()
            .get("xid")
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned()
    }

    pub async fn start_unlock(&self, session_key: &str, vehicle_key: &str) -> String {
        let client = reqwest::Client::new();
        let url = format!(
            "https://{}/{}/rems/door/unlock",
            &self.builder.base_url, &self.builder.api_url
        );
        let res = self
            .headers(client.get(url))
            .header("sid", session_key)
            .header("vinkey", vehicle_key)
            .send()
            .await
            .unwrap();

        res.headers()
            .get("xid")
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned()
    }

    pub async fn check_status(
        &self,
        session_key: &str,
        vehicle_key: &str,
        action_key: &str,
    ) -> Status {
        let client = reqwest::Client::new();
        let url = format!(
            "https://{}/{}/cmm/gts",
            &self.builder.base_url, &self.builder.api_url
        );

        #[derive(Serialize)]
        struct StatusData<'a> {
            xid: &'a str,
        }

        let data = StatusData { xid: action_key };

        let res = self
            .headers(client.post(url))
            .header("sid", session_key)
            .header("vinkey", vehicle_key)
            .json(&data)
            .send()
            .await
            .unwrap();

        #[derive(Deserialize)]
        struct StatusResponse {
            payload: Status,
        }

        let json: StatusResponse = res.json().await.unwrap();
        json.payload
    }

    fn headers(&self, req: RequestBuilder) -> RequestBuilder {
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

        req.header("content-type", "application/json;charset=UTF-8")
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
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Vehicles {
    payload: VehiclesPayload,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VehiclesPayload {
    vehicle_summary: Vec<Vehicle>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Vehicle {
    pub nick_name: String,
    pub model_name: String,
    pub trim: String,
    pub vin: String,
    pub mileage: String,
    pub vehicle_key: String,
    pub vehicle_identifier: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    alert_status: u8,
    remote_status: u8,
    ev_status: u8,
    location_status: u8,
    cal_sync_status: u8,
}
