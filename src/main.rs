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
    step1_and_step2: String,
}

async fn handle_find_my_car(query: web::Json<CarQuery>) -> impl Responder {
    info!("Received prompt: {:?}", query.prompt);

    match call_model_api(&query.prompt).await {
        Ok(api_response) => {
            info!("API Response: {:?}", api_response); // Log the full response
            match serde_json::from_str::<Vec<Value>>(&api_response) { // Parse as Vec<Value> since the response is an array
                Ok(response_array) => {
                    if let Some(response_object) = response_array.get(0) { // Get the first object in the array
                        if let Some(generated_text) = response_object["generated_text"].as_str() { // Now get generated_text from this object
                            let steps: Vec<&str> = generated_text.split("Step 3/5").collect();
                            let steps_text = steps.get(0).unwrap_or(&"").to_string();
                            HttpResponse::Ok().json(ApiResponse { step1_and_step2: steps_text })
                        } else {
                            error!("No 'generated_text' found in API response");
                            HttpResponse::InternalServerError().json("No 'generated_text' found in API response")
                        }
                    } else {
                        error!("API response array is empty");
                        HttpResponse::InternalServerError().json("API response array is empty")
                    }
                },
                Err(e) => {
                    error!("Failed to parse API response as JSON array: {:?}", e);
                    HttpResponse::InternalServerError().json("Failed to parse API response as JSON array")
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
    let model_endpoint = "https://api-inference.huggingface.co/models/google/gemma-7b";
    
    client.post(model_endpoint)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&serde_json::json!({"inputs": prompt}))
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
