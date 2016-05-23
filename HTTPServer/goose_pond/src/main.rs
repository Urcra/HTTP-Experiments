extern crate time;

use time::*;


enum RequestType {
    GET,
    HEAD,
    // Not implemented
    POST,
    PUT,
    DELTE,
    OPTIONS,
    TRACE,
}

enum HTTPHeader {
    Protocol(String),
    ProtocolVer(String),
    FilePath(String),
    Type(RequestType),
    Connection(String),
    Host(String),
    IfModifiedSince(Tm),
    IfUnmodifiedSince(Tm),

}



fn main() {
    println!("Hello, world!");
}

fn date_from_str(s: &str) -> Result<Tm, &'static str> {
    match   time::strptime(s, "%a, %d %b %Y %T %Z").or_else(|_| {
            time::strptime(s, "%A, %d-%b-%y %T %Z")}).or_else(|_| {
            time::strptime(s, "%c")}){
                Ok(t) => Ok(t),
                Err(_) => Err("Unable to parse date"),
                }
}