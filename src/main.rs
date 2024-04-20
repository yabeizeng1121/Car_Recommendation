use serde::{Deserialize, Serialize};
use warp::{http::StatusCode, Filter, Rejection, Reply};
use reqwest::Client;
use std::env;
use dotenv::dotenv;

#[derive(Deserialize, Debug)]
struct CarQuery {
    prompt: String,
}

#[derive(Serialize)]
struct ApiResponse {
    step1_and_step2: String,
}

async fn handle_find_my_car(query: CarQuery) -> Result<impl warp::Reply, warp::Rejection> {
    println!("Received prompt: {:?}", query.prompt);

    match call_model_api(query.prompt).await {
        Ok(api_response) => {
            // Parse the JSON response to extract the "generated_text" field
            if let Ok(response_json) = serde_json::from_str::<serde_json::Value>(&api_response) {
                if let Some(generated_text) = response_json["generated_text"].as_str() {
                    // Extract Step 1 and Step 2 from the generated text
                    let steps: Vec<&str> = generated_text.split("Step 3/5").collect();
                    let steps_text = steps.get(0).unwrap_or(&"").to_string();

                    // Send the extracted steps back to the client
                    Ok(warp::reply::json(&ApiResponse { step1_and_step2: steps_text }))
                } else {
                    // If parsing fails, return an error
                    Err(warp::reject::custom(ServerError))
                }
            } else {
                Err(warp::reject::custom(ServerError))
            }
        }
        Err(_) => Err(warp::reject::reject())
    }
}

async fn call_model_api(prompt: String) -> Result<String, reqwest::Error> {
    dotenv().ok(); // Load .env variables if available
    let api_key = env::var("HUGGINGFACE_API_KEY").expect("API key must be set in .env");

    let client = Client::new();
    let model_endpoint = "https://api-inference.huggingface.co/models/google/gemma-7b";

    let res = client.post(model_endpoint)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&serde_json::json!({"inputs": prompt}))
        .send()
        .await?
        .text()
        .await?;

    Ok(res)
}

// Define a custom error type for our application
#[derive(Debug)]
struct ServerError;
impl warp::reject::Reject for ServerError {}

// Custom error handling to respond with a JSON object
async fn handle_rejection(err: Rejection) -> Result<impl Reply, Rejection> {
    if err.is_not_found() {
        Ok(warp::reply::with_status(warp::reply::json(&"Not Found"), StatusCode::NOT_FOUND))
    } else if let Some(ServerError) = err.find() {
        // Here you can customize the error response
        Ok(warp::reply::with_status(
            warp::reply::json(&"Internal Server Error"),
            StatusCode::INTERNAL_SERVER_ERROR,
        ))
    } else {
        // In case we encounter an unknown error
        Err(err)
    }
}

#[tokio::main]
async fn main() {
    // Correctly pointing to index.html for the root
    let root_html_route = warp::get()
        .and(warp::path::end())
        .and(warp::fs::file("static/root/index.html"));

    // Each HTML page is served under its specific route
    let about_html_route = warp::get()
        .and(warp::path("about.html"))
        .and(warp::fs::file("static/root/about.html"));

    let html_route = warp::get()
        .and(warp::path("index.html"))
        .and(warp::fs::file("static/root/index.html"));
    
    let finder_html_route = warp::get()
        .and(warp::path("finder.html"))
        .and(warp::fs::file("static/root/finder.html"));

    let car_html_route = warp::get()
        .and(warp::path("car.html"))
        .and(warp::fs::file("static/root/car.html"));

    // Serve other static files from "static/root"
    let static_files_route = warp::get()
        .and(warp::path("static"))
        .and(warp::fs::dir("static/root"));

    let car_finder_route = warp::post()
        .and(warp::path("find_my_car"))
        .and(warp::body::json())
        .and_then(handle_find_my_car)
        .recover(handle_rejection)
        .with(warp::cors().allow_any_origin());

    // Combine all routes, ensuring the root route is properly set
    let routes = root_html_route
        .or(about_html_route)
        .or(finder_html_route)
        .or(car_html_route)
        .or(static_files_route)
        .or(html_route)
        .or(car_finder_route);

    warp::serve(routes)
        .run(([127, 0, 0, 1], 8080))
        .await;
}