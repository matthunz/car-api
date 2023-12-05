use car_api::Client;

#[tokio::main]
async fn main() {
    let client = Client::us();
    let session_key = client.login("USERNAME HERE", "PASSWORD HERE").await;
    let vehicles = client.vehicles(&session_key).await;
    let vehicle_key = &vehicles[0].vehicle_key;

    client.unlock(&session_key, vehicle_key).await;

    client.lock(&session_key, vehicle_key).await;
}
