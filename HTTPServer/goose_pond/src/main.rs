extern crate time;
extern crate mioco;

use time::*;
use std::str;

#[derive(Debug)]
enum RequestType {
    GET,
    HEAD,
    // Not implemented
    POST,
    PUT,
    DELETE,
    OPTIONS,
    TRACE,
    UNKNOWN,
}

#[derive(Debug)]
struct HTTPHeader<'a> {
    //Iinital line
    Protocol: Option<&'a str>,
    ProtocolVer: Option<&'a str>,
    FilePath: Option<&'a str>,
    Type: Option<RequestType>,
    //Header tags
    Connection: Option<&'a str>,
    Host: Option<&'a str>,
    IfModifiedSince: Option<Tm>,
    IfUnmodifiedSince: Option<Tm>,
}

impl <'a> HTTPHeader<'a> {
    fn new() -> HTTPHeader<'a>{
        HTTPHeader {
            Protocol: None,
            ProtocolVer: None,
            FilePath: None,
            Type: None,
            Connection: None,
            Host: None,
            IfModifiedSince: None,
            IfUnmodifiedSince: None
        }
    }
    fn insert_init_line(&mut self, s: &'a str) -> Result<&'static str, &'static str> {

        let mut splitted = s.split_whitespace();


        let initline;


        match (splitted.nth(0), splitted.nth(0), splitted.nth(0)) {
            (None, _, _) => return Result::Err("Invalid init line"),
            (_, None, _) => return Result::Err("Invalid init line"),
            (_, _, None) => return Result::Err("Invalid init line"),
            (Some(x), Some(y), Some(z)) => initline = (x.trim(), y.trim(), z.trim()),
        };


        let (reqtype, path, fullprot) = initline;


        self.FilePath = Some(path);
        self.Type = Some(reqtype_from_str(reqtype));


        let middle;

        match fullprot.find("/") {
            Some(i) => middle = i,
            None => return Result::Err("Invalid header format"),
        };


        let (prot, vers) = fullprot.split_at(middle);


        self.Protocol = Some(prot.trim());
        self.ProtocolVer = Some(vers[1..].trim());


        Result::Ok("OK")
    }

    fn insert_tag(&mut self, s: &'a str) -> Result<&'static str, &'static str>{
        let middle;

        match s.find(":") {
            Some(i) => middle = i,
            None => return Result::Err("Invalid header format"),
        };

        let (header, tag) = s.split_at(middle);


        match header.trim() {
            "Connection" => self.Connection = Some(tag[1..].trim()),
            "Host" => self.Host = Some(tag[1..].trim()),
            "IfModifiedSince" => match date_from_str(tag[1..].trim()) {
                            Ok(t) => self.IfModifiedSince = Some(t),
                            Err(e) => return Result::Err(e),
                            },
            "IfUnmodifiedSince" => match date_from_str(tag[1..].trim()) {
                            Ok(t) => self.IfUnmodifiedSince = Some(t),
                            Err(e) => return Result::Err(e),
                            },
            _ => return Result::Ok("Unknown header"),
        };


        Result::Ok("OK")
    }

    fn parse_req(&mut self, req: &'a [u8]) -> Result<(),()>{

        let mut len = 0;

        //Parse initialline
        match read_line(req) {
            Ok(s) => {
                self.insert_init_line(s.trim());
                len += s.len()
            },
            Err(_) => return Result::Err(()),
        };

        //Parse rest of headers
        loop {
            match read_line(&req[len..]) {
                Ok("\r\n") => break,
                Ok(s) => {
                    self.insert_tag(s.trim());
                    len += s.len()
                },
                Err(_) => return Result::Err(()),
            }
        }

        // No support for parsing the body. Since we don't support the requests using it.

        Result::Ok(())
    }
}



fn main() {
    println!("Hello, world!");

    let tmpstring = "Connection : close".to_string();
    let tmpinit = "GET /path/file.html HTTP/1.0".to_string();

    let test: &[u8] = "hej \r\n med \r\n dig \r\n lol ".as_bytes();

    let mut tmpheader = HTTPHeader::new();

    tmpheader.insert_tag(&tmpstring);
    tmpheader.insert_init_line(&tmpinit);

    let message = format!("GET {} HTTP/1.1\r\n\
                        Host: {}\r\n\
                        Connection: close\r\n\
                        User-Agent: SillyGoose/0.1\r\n\
                        \r\n", "/", "localhost");

    let mut tmpheaders = HTTPHeader::new();

    tmpheaders.parse_req(message.as_bytes());

    println!("{:?}", tmpheaders);


    println!("{:?}", tmpheader);

    println!("{:?}", read_line(test));



}

fn date_from_str(s: &str) -> Result<Tm, &'static str> {
    match   time::strptime(s, "%a, %d %b %Y %T %Z").or_else(|_| {
            time::strptime(s, "%A, %d-%b-%y %T %Z")}).or_else(|_| {
            time::strptime(s, "%c")}){
                Ok(t) => Ok(t),
                Err(_) => Err("Unable to parse date"),
                }
}

fn reqtype_from_str(req: &str) -> RequestType {
    match req {
        "GET" => RequestType::GET,
        "HEAD" => RequestType::HEAD,
        "POST" => RequestType::POST,
        "PUT" => RequestType::PUT,
        "DELETE" => RequestType::DELETE,
        "OPTIONS" => RequestType::OPTIONS,
        "TRACE" => RequestType::TRACE,
        _ => RequestType::UNKNOWN,
    }
}

fn read_line(buf: &[u8]) -> Result<&str, Option<&str>> {
    let slice_res = str::from_utf8(buf);

    let slice;

    match slice_res {
        Ok(s) => slice = s,
        Err(_) => return Result::Err(None),
    }

    let end_of_line;

    match slice.find("\r\n") {
        Some(i) => end_of_line = i,
        None => return Result::Err(Some(&slice)),
    }

    Result::Ok(&slice[..end_of_line + 2])
}