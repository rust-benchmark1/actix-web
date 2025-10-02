use actix_cors::Cors;
use actix_web::{post, App, HttpServer, HttpResponse, web, middleware::Logger, cookie::Cookie};
use mysql::*;
use mysql::prelude::*;
use actix_session::{Session, SessionMiddleware, storage::CookieSessionStore};
use actix_web::cookie::Key;

use serde::Deserialize;
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;


#[derive(Deserialize)]
struct RefreshTokenRequest {
    token: String,
}

#[post("/refreshsession")]
async fn refresh_session(
    pool: web::Data<Pool>,
    session: Session,
    body: web::Json<RefreshTokenRequest>,
) -> HttpResponse {
    let pool = pool.clone();
    let token = body.token.clone();

    let exists: bool = match web::block(move || {
        let mut conn = pool.get_conn()?;
        let row_count: u64 = conn.exec_first(
            "SELECT COUNT(*) FROM tokens WHERE token = :token",
            params! { "token" => token }
        )?.unwrap_or(0);
        Ok::<bool, mysql::Error>(row_count > 0)
    }).await {
        Ok(Ok(v)) => v,
        _ => false,
    };
    
    if exists {
        // generate a random token
        let new_token: String = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();

        // save in session
        let _ = session.insert("token", &new_token);

        // SINK CWE 1004
        // SINK CWE 614
        let cookie = Cookie::build("session_token", new_token.clone()).path("/").http_only(false).secure(false).finish();

        HttpResponse::Ok()
            .cookie(cookie)
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
            // SINK CWE 942
            .wrap(Cors::default().allow_any_origin()) // ALLOWS FOR CSRF (ANY SITE CAN CREATE A LINK TO DELETE A USER IN THIS SITE'S ROUTE)
            // session middleware
            .wrap(SessionMiddleware::builder(CookieSessionStore::default(), secret_key.clone())
                // SINK CWE 1004
                .cookie_http_only(is_http_only_active())
                .build())
            .app_data(web::Data::new(pool.clone()))
            .service(delete_user)
            .service(refresh_session)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
