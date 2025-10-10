use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::Header;
use rocket::{Request, Response};

pub struct Cors;

#[rocket::async_trait]
impl Fairing for Cors {
    fn info(&self) -> Info {
        Info {
            name: "CORS",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, req: &'r Request<'_>, res: &mut Response<'r>) {
        res.set_header(Header::new("Access-Control-Allow-Origin", "*"));
        res.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
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
    }
}
