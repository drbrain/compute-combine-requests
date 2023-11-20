use fastly::backend::Backend;
use fastly::cache::core::*;
use fastly::{Error, Request, Response};
use std::io::Write;
use std::time::Duration;

const TTL: Duration = Duration::from_secs(10);

#[fastly::main]
fn main(_req: Request) -> Result<Response, Error> {
    let result = Transaction::lookup(CacheKey::from_static(b"together")).execute();

    let lookup = match result {
        Ok(lookup) => lookup,
        Err(e) => Err(e)?,
    };

    let found = if let Some(found) = lookup.found() {
        found
    } else if lookup.must_insert() {
        let backend = Backend::from_name("http-me")?;
        let req1 = Request::get("https://http-me.glitch.me/body=1");
        let req2 = Request::get("https://http-me.glitch.me/body=2");

        let mut res1 = req1.send(&backend)?;

        let mut res2 = req2.send(&backend)?;

        let (mut writer, found) = lookup.insert(TTL).execute_and_stream_back()?;

        for chunk in res1.read_body_chunks(65536) {
            writer.write(&chunk?)?;
        }

        for chunk in res2.read_body_chunks(65536) {
            writer.write(&chunk?)?;
        }

        writer.finish()?;

        found
    } else {
        unreachable!();
    };

    let response = Response::from_body(found.to_stream()?);

    Ok(response)
}
