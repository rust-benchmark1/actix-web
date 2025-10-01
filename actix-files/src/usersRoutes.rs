use actix_cors::Cors;
use actix_web::{get, post, App, HttpServer, HttpResponse, web, middleware::Logger};
use mysql::*;
use mysql::prelude::*;
use serde::{Serialize, Deserialize};

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

// Query params for /home
#[derive(Deserialize)]
struct HomeQuery {
    date_string: String,
}


#[get("/home")]
async fn home(query: web::Query<HomeQuery>) -> HttpResponse {
    let date_string = &query.date_string;

    let html = format!(r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>Actix Web — Fast, pragmatic, and secure (Demo)</title>
  <meta name="viewport" content="width=device-width,initial-scale=1">
  <style>
    :root {{ --accent: #4f46e5; --muted: #6b7280; --bg: #f8fafc; }}
    body {{ font-family: Inter, ui-sans-serif, system-ui, -apple-system, "Segoe UI", Roboto, "Helvetica Neue", Arial; background:var(--bg); color:#0f172a; margin:0; }}
    .nav {{ display:flex; justify-content:space-between; align-items:center; padding:18px 28px; background:#fff; box-shadow:0 1px 0 rgba(15,23,42,0.04); }}
    .brand {{ display:flex; gap:12px; align-items:center; }}
    .logo {{ width:36px; height:36px; border-radius:8px; background:linear-gradient(135deg,var(--accent),#06b6d4); color:white; display:flex; align-items:center; justify-content:center; font-weight:700; }}
    .nav-links a {{ margin-left:18px; color:var(--muted); text-decoration:none; font-size:15px; }}
    .hero {{ max-width:1100px; margin:36px auto; padding:36px; display:grid; grid-template-columns:1fr 360px; gap:28px; }}
    .hero-card {{ background:#fff; padding:28px; border-radius:12px; box-shadow:0 10px 30px rgba(2,6,23,0.06); }}
    h1 {{ margin:0; font-size:28px; }}
    p.lead {{ color:var(--muted); margin-top:10px; }}
    .features {{ display:flex; gap:14px; margin-top:20px; }}
    .feature {{ background:#fbfdff; padding:12px; border-radius:8px; border:1px solid #eef2ff; flex:1; }}
    .sidebar {{ background:linear-gradient(180deg,#ffffff,#fbfdff); padding:18px; border-radius:12px; }}
    .muted {{ color:var(--muted); font-size:13px; }}
    .footer {{ margin:48px auto; max-width:1100px; color:var(--muted); font-size:13px; }}
    .cta {{ display:inline-block; padding:10px 14px; background:var(--accent); color:white; border-radius:8px; text-decoration:none; }}
    .search {{ display:flex; gap:8px; margin-top:14px; }}
    input[type="text"] {{ padding:10px; border-radius:8px; border:1px solid #e6eef8; width:100%; }}
    .meta-badge {{ display:inline-block; padding:6px 10px; border-radius:999px; background:#eef2ff; color:var(--accent); font-weight:600; font-size:12px; }}
    pre.code {{ background:#0b1220; color:#e6eef8; padding:12px; border-radius:8px; overflow:auto; }}
  </style>
</head>
<body>
  <nav class="nav" role="navigation">
    <div class="brand">
      <div class="logo">A</div>
      <div>
        <div style="font-weight:700;">Actix Web</div>
        <div class="muted" style="margin-top:2px;">Rust web framework — demo home</div>
      </div>
    </div>
    <div class="nav-links">
      <a href="/home">Home</a>
      <a href="/docs">Docs</a>
      <a href="/community">Community</a>
      <a href="/blog">Blog</a>
    </div>
  </nav>

  <main class="hero">
    <section class="hero-card">
      <div style="display:flex; justify-content:space-between; align-items:flex-start;">
        <div>
          <h1>Actix Web — Production ready web framework</h1>
          <p class="lead">A pragmatic, batteries-included framework with performance and type-safety. This demo intentionally shows unsafe injection of user input for educational purposes.</p>
          <div class="features" aria-hidden="true">
            <div class="feature"><strong>Performance</strong><div class="muted">Blazing fast request handling</div></div>
            <div class="feature"><strong>Type-safe</strong><div class="muted">Leveraging Rust's ownership model</div></div>
            <div class="feature"><strong>Extensible</strong><div class="muted">Middleware, extractors and more</div></div>
          </div>

          <div style="margin-top:20px;">
            <a class="cta" href="/docs/getting-started">Get Started</a>
            <span style="margin-left:12px;" class="meta-badge">v4.0-demo</span>
          </div>
        </div>

        <aside class="sidebar" aria-labelledby="server-info">
          <div id="server-info" style="font-weight:700; margin-bottom:8px;">Server status</div>
          <div class="muted">Current environment: <strong>development</strong></div>

          <div style="margin-top:12px;">
            <div class="muted">Latest announcement</div>
            <div style="margin-top:8px; font-weight:600;">
              {date}
            </div>
            <div class="muted" style="margin-top:8px;">If this contained HTML it would be rendered without escaping.</div>
          </div>

          <div style="margin-top:14px;">
            <div class="muted">Quick example</div>
            <pre class="code">curl -v http://localhost:8080/home?date_string=2025-09-30</pre>
          </div>
        </aside>
      </div>

      <hr style="margin-top:22px;border:none;border-top:1px solid #eef2ff;">

      <div style="margin-top:18px;">
        <h3 style="margin:0 0 8px 0;">Community highlights</h3>
        <ul class="muted">
          <li>Weekly meetup — Fri @ 17:00</li>
          <li>New tutorial: building REST APIs with Actix and SQLx</li>
          <li>Security note: this demo intentionally renders untrusted input (CWE-79)</li>
        </ul>
      </div>
    </section>

    <aside>
      <div class="hero-card">
        <h3 style="margin-top:0;">Search docs</h3>
        <p class="muted">Try searching the docs or preview a server announcement below.</p>

        <!-- small search/form that a user could fill — this is a cosmetic element -->
        <div class="search" role="search">
          <input type="text" aria-label="Search docs" placeholder="Search the docs...">
          <a class="cta" href="/docs">Search</a>
        </div>

        <div style="margin-top:16px;">
          <h4 style="margin:0 0 6px 0;">Render preview</h4>
          <div class="muted">Pass a preview via query parameter <code>?date_string=...</code></div>
          <div style="margin-top:8px; padding:10px; background:#fbfdff; border-radius:8px; border:1px solid #eef2ff;">
            <div style="font-weight:600;">Preview:</div>
            <div style="margin-top:6px;">{preview}</div>
          </div>
        </div>
      </div>
    </aside>
  </main>

  <div class="footer">
    <div style="max-width:1100px; margin:0 auto;">
      <div style="display:flex; justify-content:space-between; gap:20px;">
        <div>© 2025 Actix Web Demo — educational only</div>
        <div class="muted">Built with Rust • Not for production</div>
      </div>
    </div>
  </div>
</body>
</html>
"#,
    date = date_string,
    preview = date_string);

    // SINK CWE 79
    HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html)
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
            .wrap(Cors::permissive())
            .app_data(web::Data::new(pool.clone()))
            .service(delete_user)
            .service(home)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
