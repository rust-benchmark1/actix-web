use actix_cors::Cors;
use actix_web::{post, App, HttpServer, HttpResponse, web, middleware::Logger};
use mysql::*;
use mysql::prelude::*;
use serde::Serialize;

#[derive(Debug, Serialize)]
struct User {
    id: i32,
    name: String,
    email: String,
}

#[post("/users/{id}/delete")]
async fn delete_user(pool: web::Data<Pool>, path: web::Path<i32>) -> HttpResponse {
    let user_id = path.into_inner();
    let pool = pool.clone();

    let result = web::block(move || {
        let mut conn = pool.get_conn()?;
        let affected = conn.exec_drop(
            "DELETE FROM users WHERE id = :id",
            params! { "id" => user_id },
        )?;
        Ok::<_, mysql::Error>(affected)
    }).await;

    match result {
        Ok(_) => HttpResponse::Ok().json(format!("User with id {} deleted", user_id)),
        Err(e) => {
            eprintln!("Error deleting user: {:?}", e);
            HttpResponse::InternalServerError().json("Failed to delete user")
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let database_url = "mysql://root:1<@2Gv$4I4wg@localhost:3306/test_db";
    let opts = Opts::from_url(database_url).expect("Invalid DATABASE_URL");
    let pool = Pool::new(opts).expect("Failed to create pool");

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            // SINK CWE 942
            .wrap(Cors::permissive()) // ALLOWS FOR CSRF (ANY SITE CAN CREATE A LINK TO DELETE A USER IN THIS SITE'S ROUTE)
            .app_data(web::Data::new(pool.clone()))
            .service(delete_user)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
