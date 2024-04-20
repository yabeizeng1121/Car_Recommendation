use actix_web::{web, App, HttpServer, HttpResponse, Responder, middleware};
use actix_files as fs;
use serde::{Deserialize, Serialize};
use reqwest::Client;
use dotenv::dotenv;
use std::env;
use log::{info, error};
use env_logger;
use serde_json::{Value, json};

#[derive(Deserialize, Debug)]
struct CarQuery {
    prompt: String,
}

#[derive(Serialize)]
struct ApiResponse {
    steps: String, // Changed from 'step1_and_step2' to 'steps' to reflect full steps handling
}

async fn handle_find_my_car(query: web::Json<CarQuery>) -> impl Responder {
    info!("Received prompt: {:?}", query.prompt);

    match call_model_api(&query.prompt).await {
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

async fn call_model_api(prompt: &str) -> Result<String, reqwest::Error> {
    dotenv().ok();
    let api_key = env::var("HUGGINGFACE_API_KEY").expect("API key must be set in .env");
    
    let client = Client::new();
    let model_endpoint = "https://api.replicate.com/v1/models/meta/meta-llama-3-70b-instruct/predictions";
    
    client.post(model_endpoint)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&json!({"input": {"prompt": prompt}}))  // Updated the structure to match what the API expects
        .send()
        .await?
        .text()
        .await
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    HttpServer::new(|| {
        App::new()
            .wrap(middleware::Logger::default()) // Logging middleware
            .service(web::resource("/find_my_car").route(web::post().to(handle_find_my_car)))
            .service(fs::Files::new("/static", "static/root").show_files_listing())
            .service(fs::Files::new("/imgs", "static/root/imgs").show_files_listing())
            .service(fs::Files::new("/css", "static/root/css").show_files_listing())  // Serving CSS files
            // Setup routes for static HTML files
            .service(web::resource("/").route(web::get().to(|| async {
                fs::NamedFile::open("static/root/index.html")
            })))
            .service(web::resource("/index.html").route(web::get().to(|| async {
                fs::NamedFile::open("static/root/index.html")
            })))
            .service(web::resource("/about.html").route(web::get().to(|| async {
                fs::NamedFile::open("static/root/about.html")
            })))
            .service(web::resource("/finder.html").route(web::get().to(|| async {
                fs::NamedFile::open("static/root/finder.html")
            })))
            .service(web::resource("/car.html").route(web::get().to(|| async {
                fs::NamedFile::open("static/root/car.html")
            })))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
