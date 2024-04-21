use actix_web::{web, App, HttpServer, HttpResponse, Responder, middleware};
use actix_files as fs;
use serde::{Deserialize, Serialize};
use reqwest::Client;
use dotenv::dotenv;
use std::env;
use log::{info, error};
use env_logger;
use serde_json::Value;

#[derive(Deserialize, Debug)]
struct CarQuery {
    prompt: String,
}

#[derive(Serialize)]
struct ApiResponse {
    steps: String,
}

#[derive(Deserialize)]
struct InitialApiResponse {
    id: String,
    status: String,
    urls: ApiUrls,
}

#[derive(Deserialize)]
struct ApiUrls {
    get: String,
}

async fn handle_find_my_car(client: web::Data<Client>, query: web::Json<CarQuery>) -> impl Responder {
    info!("Received prompt: {:?}", query.prompt);

    // Pass the shared Client instance to call_model_api
    match call_model_api(&query.prompt, &client).await {
        Ok(api_response) => {
            info!("API Response: {:?}", api_response);
            match serde_json::from_str::<Value>(&api_response) {
                Ok(response_value) => {
                    if let Some(output) = response_value["output"].as_array() {
                        let steps_text: Vec<String> = output.iter().map(|s| s.as_str().unwrap_or("").to_string()).collect();
                        HttpResponse::Ok().json(ApiResponse { steps: steps_text.join("") })
                    } else {
                        error!("No 'output' found in API response");
                        HttpResponse::InternalServerError().json("No 'output' found in API response")
                    }
                },
                Err(e) => {
                    error!("Failed to parse API response: {:?}", e);
                    HttpResponse::InternalServerError().json(format!("Failed to parse API response: {:?}", e))
                }
            }
        },
        Err(e) => {
            error!("Failed to call model API: {:?}", e);
            HttpResponse::InternalServerError().json("Failed to call model API")
        }
    }
}

async fn call_model_api(prompt: &str, client: &Client) -> Result<String, String> {
    dotenv().ok();
    let api_key = env::var("HUGGINGFACE_API_KEY")
        .map_err(|e| format!("Error getting API key: {}", e))?;
    
    let model_endpoint = "https://api.replicate.com/v1/models/mistralai/mistral-7b-instruct-v0.2/predictions";
    
    let response = client.post(model_endpoint)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&serde_json::json!({"input": {"max_new_tokens": 1000,"prompt": prompt}}))
        .send()
        .await
        .map_err(|e| format!("Network request error: {}", e))?;
    
    let response_text = response.text().await
        .map_err(|e| format!("Failed to read response text: {}", e))?;
    
    let initial_api_response: InitialApiResponse = serde_json::from_str(&response_text)
        .map_err(|e| format!("Failed to deserialize initial API response: {}", e))?;
    
    let status_url = initial_api_response.urls.get;
    let mut final_result = String::new();

    loop {
        let status_response = client.get(&status_url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
            .map_err(|e| format!("Network request error: {}", e))?
            .text()
            .await
            .map_err(|e| format!("Failed to read status response text: {}", e))?;

        if status_response.contains("\"status\":\"succeeded\"") {
            final_result = status_response;
            break;
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }

    Ok(final_result)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init();

    // Initialize an instance of Client to be used across the application
    let client = Client::new();

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(client.clone())) // Changed to use .app_data
            .wrap(middleware::Logger::default()) // Logging middleware
            .service(web::resource("/find_my_car").route(web::post().to(handle_find_my_car))) // Define your route and handler
            .service(fs::Files::new("/static", "static/root").show_files_listing())
            .service(fs::Files::new("/imgs", "static/root/imgs").show_files_listing())
            .service(fs::Files::new("/css", "static/root/css").show_files_listing())  // Serving CSS files
            // Routes for static HTML files
            .service(web::resource("/").route(web::get().to(|| async { fs::NamedFile::open("static/root/index.html") })))
            .service(web::resource("/index.html").route(web::get().to(|| async { fs::NamedFile::open("static/root/index.html") })))
            .service(web::resource("/about.html").route(web::get().to(|| async { fs::NamedFile::open("static/root/about.html") })))
            .service(web::resource("/finder.html").route(web::get().to(|| async { fs::NamedFile::open("static/root/finder.html") })))
            .service(web::resource("/car.html").route(web::get().to(|| async { fs::NamedFile::open("static/root/car.html") })))
    })
    .bind("127.0.0.1:8080")? // Bind to the server to localhost:8080
    .run() // Run the server
    .await // Await the server's execution
}