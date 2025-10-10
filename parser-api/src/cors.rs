use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::{ContentType, Header, Method, Status};
use rocket::{Request, Response};

pub struct Cors;

fn is_allowed_origin(origin: &str) -> bool {
    matches!(
        origin,
        "http://localhost:8080" | "http://127.0.0.1:8080" | "https://e621scraper.duckdns.org"
    )
}

#[rocket::async_trait]
impl Fairing for Cors {
    fn info(&self) -> Info {
        Info {
            name: "CORS",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, req: &'r Request<'_>, res: &mut Response<'r>) {
        let origin = req.headers().get_one("Origin");

        // Only set ACAO when we have a legit allowed Origin (avoid "*" with credentials)
        if let Some(o) = origin {
            if is_allowed_origin(o) {
                res.set_header(Header::new("Access-Control-Allow-Origin", o));
                res.set_header(Header::new("Vary", "Origin"));
                res.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
            }
        }

        // Methods
        res.set_header(Header::new(
            "Access-Control-Allow-Methods",
            "GET, POST, PUT, PATCH, DELETE, OPTIONS, HEAD",
        ));

        // Reflect requested headers for preflight; fall back to a sane list
        let req_headers = req
            .headers()
            .get_one("Access-Control-Request-Headers")
            .unwrap_or("Authorization, Accept, Content-Type");
        res.set_header(Header::new("Access-Control-Allow-Headers", req_headers));

        // Cache preflight (optional)
        res.set_header(Header::new("Access-Control-Max-Age", "86400"));

        // Short-circuit preflight cleanly
        if req.method() == Method::Options {
            res.set_status(Status::NoContent);
            res.set_header(ContentType::Plain);
            res.set_sized_body(0, std::io::Cursor::new(""));
        }
    }
}
