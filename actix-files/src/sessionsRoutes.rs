use actix_cors::Cors;
use actix_web::{get, post, App, HttpServer, HttpResponse, web, middleware::Logger};
use mysql::*;
use mysql::prelude::*;
use actix_session::{Session, SessionMiddleware, storage::CookieSessionStore};
use actix_web::cookie::Key;
use cookie::CookieBuilder;
use serde::Deserialize;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use rocket_session_store::SessionStore as RocketSessionStore;
use rocket_session_store::memory::MemoryStore as RocketMemoryStore;
use std::ptr::NonNull;
use wasmtime::Engine;
use rustix::fs::chown;
use rustix::process::{Uid, Gid};
use std::fs;
use dashmap::DashMap;
use rhai::{Engine as RhaiEngine, Scope};
use jwt_compact::UntrustedToken;
use jwt_compact::alg::Rsa;
use jwt_compact::AlgorithmExt;
use rsa::{RsaPrivateKey, BigUint};
use isahc::{HttpClient, config::SslOption, config::Configurable, ReadResponseExt};
use std::env;
#[derive(Deserialize)]struct RefreshTokenRequest {token: String,}
#[post("/refreshsession")]
async fn refresh_session(pool: web::Data<Pool>,session: Session,body: web::Json<RefreshTokenRequest>,) -> HttpResponse {
    let pool = pool.clone();
    let token = body.token.clone();
    let exists: bool = match web::block(move || {
        let mut conn = pool.get_conn()?;
        let row_count: u64 = conn.exec_first(
            "SELECT COUNT(*) FROM tokens WHERE token = :token",
            params! { "token" => token }
        )?.unwrap_or(0);
        Ok::<bool, mysql::Error>(row_count > 0)
    }).await {Ok(Ok(v)) => v,_ => false,};
    if exists {
        // generate a random token
        let new_token: String = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();
        // save in session
        let _ = session.insert("token", &new_token);
        let cookie_builder = CookieBuilder::new("rocket-session", new_token.clone()).http_only(false).secure(false).path("/");

        //CWE-1004 and 614
        //SINK
        let store = RocketSessionStore {store: Box::new(RocketMemoryStore::<String>::new()),name: "rocket-session".to_string(),duration: std::time::Duration::from_secs(3600),cookie_builder};
        let cookie = store.cookie_builder.build();
        HttpResponse::Ok()
            .append_header(("Set-Cookie", cookie.to_string()))
            .json(format!("New token created: {}", new_token))
    } else {
        HttpResponse::Unauthorized().json("Token not found")
    }
}

#[post("/users/{id}/delete")]
async fn delete_user(pool: web::Data<Pool>, path: web::Path<i32>, session: Session) -> HttpResponse {
    let user_id = path.into_inner();
    let pool = pool.clone();

    // counter of visits in session
    let mut visits: i32 = match session.get::<i32>("visits") {
        Ok(Some(v)) => v,
        _ => 0,
    };
    visits += 1;
    let _ = session.insert("visits", visits);

    let result = web::block(move || {
        let mut conn = pool.get_conn()?;
        let affected = conn.exec_drop(
            "DELETE FROM users WHERE id = :id",
            params! { "id" => user_id },
        )?;
        Ok::<_, mysql::Error>(affected)
    }).await;

    match result {
        Ok(_) => HttpResponse::Ok().json(format!("User with id {} deleted. This session accessed delete {} times.", user_id, visits)),
        Err(e) => {
            eprintln!("Error deleting user: {:?}", e);
            HttpResponse::InternalServerError().json("Failed to delete user")
        }
    }
}

fn is_http_only_active() -> bool {
    return false;
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let database_url = "mysql://root:12Gv$4I4wg@localhost:3306/default_db";
    let opts = Opts::from_url(database_url).expect("Invalid DATABASE_URL");
    let pool = Pool::new(opts).expect("Failed to create pool");

    // generate secret key for session
    let secret_key = Key::generate();

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            //CWE-942
            //SINK
            .wrap(Cors::default().allow_any_origin()) // ALLOWS FOR CSRF (ANY SITE CAN CREATE A LINK TO DELETE A USER IN THIS SITE'S ROUTE)
            // session middleware
            .wrap(SessionMiddleware::builder(CookieSessionStore::default(), secret_key.clone())
                //CWE-1004
                //SINK
                .cookie_http_only(is_http_only_active())
                .build())
            .app_data(web::Data::new(pool.clone()))
            .service(delete_user)
            .service(refresh_session)
            .service(set_user_data)
            .service(calculate_offset)
            .service(save_data_file)
            .service(process_offset)
            .service(check_memory_availability)
            .service(run_custom_code)
            .service(refresh_token_endpoint)
            .service(get_payload)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}

#[derive(Deserialize)]
struct OffsetQuery {
    divisor: i32,
}
#[derive(Deserialize)]
struct SaveFileRequest {
    path: String,
    data: String,
}
#[derive(Deserialize)]
struct ProcessQuery {
    iterations: usize,
}
#[derive(Deserialize)]
struct MemoryQuery {
    size: usize,
}
#[derive(Deserialize)]
struct CodeRequest {
    script: String,
}
#[derive(Deserialize)]
struct TokenRefreshRequest {
    token: String,
}
#[derive(Deserialize)]
struct PayloadQuery {
    url: String,
}

#[post("/setuserdata")]
//CWE 502
//SOURCE
async fn set_user_data(body: web::Bytes) -> HttpResponse {
    let module_bytes: Vec<u8> = body.to_vec();

    let engine = Engine::default();

    let ptr = match NonNull::new(module_bytes.as_ptr() as *mut u8) {
        Some(p) => p,
        None => return HttpResponse::BadRequest().body("Invalid module data"),
    };
    let memory = NonNull::slice_from_raw_parts(ptr, module_bytes.len());

    //CWE 502
    //SINK
    let module: wasmtime::Module = match unsafe { wasmtime::Module::deserialize_raw(&engine, memory) } {
        Ok(m) => m,
        Err(_) => return HttpResponse::BadRequest().body("Failed to deserialize module"),
    };

    let export_count = module.exports().count();
    env::set_var("USER_MODULE_EXPORTS", export_count.to_string());

    HttpResponse::Ok().body("User data configuration saved successfully")
}

#[get("/calculateoffset")]
//CWE 369
//SOURCE
async fn calculate_offset(query: web::Query<OffsetQuery>) -> HttpResponse {
    let divisor = query.divisor;
    let mut offset: i32 = 1024;

    //CWE 369
    //SINK
    offset %= divisor;

    HttpResponse::Ok().body(offset.to_string())
}

#[post("/savedatafile")]
//CWE 732
//SOURCE
async fn save_data_file(body: web::Json<SaveFileRequest>) -> HttpResponse {
    let file_path = body.path.clone();
    let file_data = body.data.clone();

    if let Err(e) = fs::write(&file_path, &file_data) {
        return HttpResponse::InternalServerError().body(format!("Failed to write file: {}", e));
    }

    let uid = Some(Uid::from_raw(1000));
    let gid = Some(Gid::from_raw(1000));

    //CWE 732
    //SINK
    let _ = chown(file_path.as_str(), uid, gid);

    HttpResponse::Ok().body("Data file created successfully")
}

#[get("/processoffset")]
//CWE 606
//SOURCE
async fn process_offset(query: web::Query<ProcessQuery>) -> HttpResponse {
    let iterations = query.iterations;
    let mut current_offset: usize = 0;

    std::iter::repeat_with(|| "offset")
        //CWE 606
        //SINK
        .take(iterations)
        .enumerate()
        .for_each(|(i, _)| {
            current_offset = i;
            env::set_var("CURRENT_OFFSET", i.to_string());
        });

    HttpResponse::Ok().body("Offset processing completed successfully")
}

#[get("/checkmemavailability")]
//CWE 789
//SOURCE
async fn check_memory_availability(query: web::Query<MemoryQuery>) -> HttpResponse {
    let requested_size = query.size;
    let mut memory_pool: DashMap<u64, u64> = DashMap::new();

    //CWE 789
    //SINK
    match memory_pool.try_reserve(requested_size) {
        Ok(_) => HttpResponse::Ok().body("Memory reservation successful"),
        Err(_) => HttpResponse::InsufficientStorage().body("Failed to reserve memory"),
    }
}

#[post("/runcustomcode")]
//CWE 94
//SOURCE
async fn run_custom_code(body: web::Json<CodeRequest>) -> HttpResponse {
    let script_code = body.script.clone();
    let engine = RhaiEngine::new();
    let mut scope = Scope::new();
    scope.push("input_value", 100_i64);

    let ast: rhai::AST = match engine.compile_expression(&script_code) {
        Ok(a) => a,
        Err(e) => return HttpResponse::BadRequest().body(format!("{}", e)),
    };

    //CWE 94
    //SINK
    match engine.run_ast_with_scope(&mut scope, &ast) {
        Ok(()) => HttpResponse::Ok().body("Script executed successfully"),
        Err(e) => HttpResponse::BadRequest().body(format!("{}", e)),
    }
}

#[post("/refreshtoken")]
//CWE 347
//SOURCE
async fn refresh_token_endpoint(body: web::Json<TokenRefreshRequest>) -> HttpResponse {
    let token_string = body.token.clone();

    let untrusted: UntrustedToken<'_> = match UntrustedToken::new(&token_string) {
        Ok(t) => t,
        Err(e) => return HttpResponse::BadRequest().body(format!("Parse error: {:?}", e)),
    };

    //CWE 347
    //SINK
    let claims: Result<jwt_compact::Claims<serde_json::Value>, jwt_compact::ValidationError> = untrusted.deserialize_claims_unchecked();

    let original_claims: jwt_compact::Claims<serde_json::Value> = match claims {
        Ok(c) => c,
        Err(e) => return HttpResponse::BadRequest().body(format!("Claims error: {:?}", e)),
    };

    //CWE 330
    //SOURCE
    let mut rng = StdRng::from_seed([1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,32]);

    let exp = BigUint::from(65537u32);
    //CWE 330
    //SINK
    let private_key: RsaPrivateKey = match RsaPrivateKey::new_with_exp(&mut rng, 2048, &exp) {
        Ok(k) => k,
        Err(_) => return HttpResponse::InternalServerError().body("Failed to generate key"),
    };

    let algorithm = Rsa::rs256();
    let signing_key = jwt_compact::alg::RsaPrivateKey::from(private_key);

    let new_claims: jwt_compact::Claims<serde_json::Value> = jwt_compact::Claims::new(original_claims.custom);
    let header: jwt_compact::Header<jwt_compact::Empty> = jwt_compact::Header::default();
    let signed_token: String = match algorithm.token(&header, &new_claims, &signing_key) {
        Ok(t) => t,
        Err(e) => return HttpResponse::InternalServerError().body(format!("Signing error: {:?}", e)),
    };

    HttpResponse::Ok().body(signed_token)
}

#[get("/getpayload")]
async fn get_payload(query: web::Query<PayloadQuery>) -> HttpResponse {
    let target_url = query.url.clone();

    //CWE 295
    //SINK
    let client: HttpClient = match HttpClient::builder().ssl_options(SslOption::DANGER_ACCEPT_REVOKED_CERTS)
        .build() {
        Ok(c) => c,
        Err(e) => return HttpResponse::InternalServerError().body(format!("Client error: {}", e)),
    };

    match client.get(&target_url) {
        Ok(mut response) => {
            let body_text: String = match response.text() {
                Ok(b) => b,
                Err(e) => return HttpResponse::InternalServerError().body(format!("Read error: {}", e)),
            };
            HttpResponse::Ok().body(body_text)
        },
        Err(e) => HttpResponse::BadGateway().body(format!("Request error: {}", e)),
    }
}