use actix_web::{web, App, HttpServer, HttpResponse, Responder, middleware};
use actix_files as fs;
use serde::{Deserialize, Serialize};
use reqwest::Client;
use dotenv::dotenv;
use std::env;

#[derive(Deserialize, Debug)]
struct CarQuery {
    prompt: String,
}

#[derive(Serialize)]
struct ApiResponse {
    step1_and_step2: String,
}

async fn handle_find_my_car(query: web::Json<CarQuery>) -> impl Responder {
    println!("Received prompt: {:?}", query.prompt);
    
    match call_model_api(&query.prompt).await {
        Ok(api_response) => {
            if let Ok(response_json) = serde_json::from_str::<serde_json::Value>(&api_response) {
                if let Some(generated_text) = response_json["generated_text"].as_str() {
                    let steps: Vec<&str> = generated_text.split("Step 3/5").collect();
                    let steps_text = steps.get(0).unwrap_or(&"").to_string();
                    HttpResponse::Ok().json(ApiResponse { step1_and_step2: steps_text })
                } else {
                    HttpResponse::InternalServerError().finish()
                }
            } else {
                HttpResponse::InternalServerError().finish()
            }
        },
        Err(_) => HttpResponse::BadRequest().finish()
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
