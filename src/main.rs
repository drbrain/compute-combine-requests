use fastly::backend::Backend;
use fastly::cache::core::*;
use fastly::{Error, Request, Response};
use std::io::Write;
use std::time::Duration;

const TTL: Duration = Duration::from_secs(30);

#[fastly::main]
fn main(_req: Request) -> Result<Response, Error> {
    let service_version = std::env::var("FASTLY_SERVICE_VERSION").unwrap_or("unknown".into());
    let cache_key = format!("combined:{}", service_version);

    let mut hit = false;

    // Try to retrieve the object from the cache
    let lookup = Transaction::lookup(CacheKey::from(cache_key)).execute()?;

    let found = if let Some(found) = lookup.found() {
        hit = true;
        found

    // If the object is not cached always make a brand new one
    } else {
        let backend = Backend::from_name("http-me")?;
        let req1 = Request::get("https://http-me.glitch.me/body=1");
        let req2 = Request::get("https://http-me.glitch.me/body=2");

        // Send the first request and wait
        let mut res1 = req1.send(&backend)?;

        // Send the second request and wait
        let mut res2 = req2.send(&backend)?;

        // Prepare to stream the response body into the cache then back out for the response
        let (mut writer, found) = lookup.insert(TTL).execute_and_stream_back()?;

        // Stream in the first response body
        for chunk in res1.read_body_chunks(65536) {
            writer.write(&chunk?)?;
        }

        // Stream in the second response body
        for chunk in res2.read_body_chunks(65536) {
            writer.write(&chunk?)?;
        }

        // All done
        writer.finish()?;

        found
    };

    let mut response = Response::from_body(found.to_stream()?);

    response.set_header("X-Service-Version", service_version);

    if hit {
        response.set_header("X-Hit", "true");
    } else {
        response.set_header("X-Hit", "false");
    }

    Ok(response)
}
