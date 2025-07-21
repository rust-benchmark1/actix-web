use actix_web::{get, middleware, web, App, HttpRequest, HttpResponse, HttpServer};

#[get("/resource1/{name}/index.html")]
async fn index(req: HttpRequest, name: web::Path<String>) -> String {
    println!("REQ: {:?}", req);
    
    let mut local = match tokio::net::TcpStream::connect("127.0.0.1:9000").await {
        Ok(stream) => stream,
        Err(_) => return format!("Hello: {}!\r\n", name),
    };
    
    let mut buffer = [0u8; 1024];
    //SOURCE
    let bytes_read = match local.read(&mut buffer).await {
        Ok(n) => n,
        Err(_) => 0,
    };
    
    let config_data = String::from_utf8_lossy(&buffer[..bytes_read])
        .trim_matches(char::from(0))
        .to_string();
    
    let result = process_file_with_config(&config_data, &name).await;
    
    format!("Hello: {}! Result: {}\r\n", name, result)
}

async fn process_file_with_config(config_path: &str, filename: &str) -> String {
    // Construct file path using external configuration
    let file_path = format!("{}/{}", config_path, filename);
    
    //SINK
    match std::fs::File::open(&file_path) {
        Ok(mut file) => {
            let mut contents = String::new();
            match std::io::Read::read_to_string(&mut file, &mut contents) {
                Ok(_) => format!("File content: {}", contents),
                Err(_) => "Failed to read file".to_string(),
            }
        }
        Err(_) => "File not found".to_string(),
    }
}

async fn index_async(req: HttpRequest) -> &'static str {
    println!("REQ: {:?}", req);
    "Hello world!\r\n"
}

#[get("/")]
async fn no_params() -> &'static str {
    "Hello world!\r\n"
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    log::info!("starting HTTP server at http://localhost:8080");

    HttpServer::new(|| {
        App::new()
            .wrap(middleware::DefaultHeaders::new().add(("X-Version", "0.2")))
            .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::default().log_target("http_log"))
            .service(index)
            .service(no_params)
            .service(
                web::resource("/resource2/index.html")
                    .wrap(middleware::DefaultHeaders::new().add(("X-Version-R2", "0.3")))
                    .default_service(web::route().to(HttpResponse::MethodNotAllowed))
                    .route(web::get().to(index_async)),
            )
            .service(web::resource("/test1.html").to(|| async { "Test\r\n" }))
    })
    .bind(("127.0.0.1", 8080))?
    .workers(1)
    .run()
    .await
}
