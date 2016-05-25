#![feature(fs_time)] 
extern crate time;
extern crate clap;
extern crate regex;


use std::io::prelude::*;
use std::sync::Arc;
use std::path::PathBuf;
use std::path::Path;
use std::fs::File;
use clap::{App, Arg, SubCommand};
use time::*;
use std::str;
use std::net::SocketAddr;
use std::str::FromStr;
//use std::io::{self, Write, Read};
use std::net::{TcpListener, TcpStream};
use std::thread;
use regex::Regex;
//use mioco::tcp::TcpListener;


#[derive(Debug)]
struct ServerConfig {
    fileroot: String,
    address: SocketAddr,
}


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
            "If-Modified-Since" => match date_from_str(tag[1..].trim()) {
                            Ok(t) => self.IfModifiedSince = Some(t),
                            Err(e) => return Result::Err(e),
                            },
            "If-Content-Length: 14\rUnmodified-Since" => match date_from_str(tag[1..].trim()) {
                            Ok(t) => self.IfUnmodifiedSince = Some(t),
                            Err(e) => return Result::Err(e),
                            },
            _ => return Result::Ok("Unknown header"),
        };


        Result::Ok("OK")
    }

    fn parse_req(&mut self, req: &'a [u8]) -> Result<(),&'static str>{

        let mut len = 0;

        //Parse initialline
        match read_line(req) {
            Ok(s) => {
                self.insert_init_line(s.trim());
                len += s.len()
            },
            Err(_) => return Result::Err("Missing line ending"),
        };

        //Parse rest of headers
        loop {
            match read_line(&req[len..]) {
                Ok("\r\n") => break,
                Ok(s) => {
                    match self.insert_tag(s.trim()){
                        Ok(_) => len += s.len(),
                        Err(e) => return Result::Err(e),
                    };
                    
                },
                Err(_) => return Result::Err("Missing line ending"),
            }
        }

        // No support for parsing the body. Since we don't support the requests using it.

        Result::Ok(())
    }
}
// Make my own
const RESPONSE: &'static str = "HTTP/1.1 200 OK\r
Content-Length: 14\r
Connection: close\r
\r
Hello World\r
\r";

const RESPONSE_PERS: &'static str = "HTTP/1.1 200 OK\r
Content-Length: 14\r
\r
Hello World\r
\r";

const RESPONSE_404: &'static str = "HTTP/1.1 404 Not Found\r
Content-Length: 18\r
Connection: close\r
\r
Hello World_404\r
\r";


const RESPONSE_NULL: &'static str = "\0";


fn handle_client(mut stream: TcpStream, fileroot: &Arc<String>) {

    //println!("{:?}", fileroot);

    let ref derefed = **fileroot;
    let mut path = PathBuf::from(derefed);

    println!("{:?}", path);

    let mut buffer = [0u8; 2048];
    let mut foo = Vec::new();
    
    let mut headers = HTTPHeader::new();

    

    let _ = stream.read(&mut buffer);

    match headers.parse_req(&buffer) {
        Ok(_) => {
            match headers.Type {
                Some(RequestType::HEAD) => (),
                Some(RequestType::GET) => (),
                Some(_) => {
                    send_error(stream, "501 Not Implemented", "The server does not support the method");
                    return;
                },
                None => {
                    send_error(stream, "400 Bad Request", "Could not find method");
                    return;
                },
            }
        },
        Err(e) => {
            send_error(stream, "400 Bad Request", e);
            return;
        },
    };


    //Require host headers if they use version 1.1
    match headers.Host {
        None => match headers.ProtocolVer {
            Some("1.1") => {
                send_error(stream, "400 Bad Request", "Missing host header");
                return;
            },
            _ => (),
        },
        _ => (),
    };


    if let Some(givenpath) = headers.FilePath {

        //Support requests from future versions of HTTP, that sends absolute paths
        match headers.ProtocolVer {
            Some("0.9") | Some("1.0") | Some("1.1") => path.push(&givenpath[1..]),
            _ => {
                let relpath = from_absolute(&givenpath);
                path.push(&relpath[1..]);
            }
        };   
        
    } else {
        println!("should not happen");
    }


    let metadata;
    let mut filehandle;

    if let Ok(mut f) = File::open(path) {


        match f.metadata() {
            Ok(m) => metadata = m,
            Err(_) => { 
                send_error(stream, "404 Not Found", "No such file on the server");
                return;
            },
        };

        filehandle = f;


    } else {
        send_error(stream, "404 Not Found", "No such file on the server");
        return;
    }

    if let Some(time) = headers.IfModifiedSince {
        let lastmod = metadata.modified().unwrap();

        let currenttime = time::get_time();
        let timesinceifmod = (currenttime - time.to_timespec()).to_std().unwrap();
        let timesincemod = lastmod.elapsed().unwrap();

        if timesinceifmod < timesincemod {
            //Not modified since client last saw it
            send_error(stream, "304 Not Modified", "The requested file has not been modified");
            return;
        }
    }

    if let Some(time) = headers.IfUnmodifiedSince {
        let lastmod = metadata.modified().unwrap();

        let currenttime = time::get_time();
        let timesinceifmod = (currenttime - time.to_timespec()).to_std().unwrap();
        let timesincemod = lastmod.elapsed().unwrap();

        if timesinceifmod > timesincemod {
            //Modified since client last saw it
            send_error(stream, "412 Precondition Failed", "The requested file has been modified");
            return;
        }

    }


    if metadata.is_file() {

        match filehandle.read_to_end(&mut foo) {
            Ok(o) => stream.write_all(&foo).unwrap(),
            Err(e) => send_error(stream, "404 Not Found", "No such file on the server"),
        };
    }


    // TCP connection closes after this scope
}



fn from_absolute(abspath: &str) -> &str {
    let url_regex = Regex::new(r"^(([^:/?#]+):)?(//([^/?#]*))?([^?#]*)(\?([^#]*))?(#(.*))?").unwrap();

    let captured = url_regex.captures(abspath).unwrap();

    //fix crashing
    captured.at(5).expect("Malformed url")
}

fn send_error(mut stream: TcpStream, error_code: &str, reason: &str) {

    let body = format!("<html>\r\n\
            <body>\r\n\
            <h2>Error {}</h2>\r\n\
            {}\r\n\
            <html>\r\n\
            <body>\r\n", error_code, reason);

    let msg = format!("HTTP/1.1 {}\r\n\
            Content-type: text-html\r\n\
            Connection: close\r\n\
            Content-Length: {}\r\n\
            \r\n\
            {}", error_code, body.len(), body);

    stream.write_all(msg.as_bytes());
}




fn main() {

    //Command line argument parsing
    let arguments = App::new("Goose_pond")
                        .version("0.1")
                        .author("Christian N")
                        .about("A simple http file server")
                        .arg(Arg::with_name("adress")
                                    .short("a")
                                    .long("adress")
                                    .value_name("adress")
                                    .help("Sets the adress where the servers listens on")
                                    .takes_value(true))
                        .arg(Arg::with_name("path")
                                    .required(true)
                                    .help("The path to the root folder for the server")
                                    .index(1))
                        .arg(Arg::with_name("port")
                                    .required(true)
                                    .help("The port the server should listen on")
                                    .index(2))
                        .get_matches();


    let address = arguments.value_of("adress").unwrap_or("0.0.0.0");
    let port = arguments.value_of("port").unwrap();
    let serverroot = arguments.value_of("path").unwrap();

    let path = Arc::new(serverroot.to_string());

    let combined: SocketAddr = match [address, port].join(":").parse() {
        Ok(a) => a,
        Err(_) => {
            println!("{}:{} is not a valid adress", address, port);
            return;
        }
    };





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

    //println!("{:?}", tmpheaders);


    //println!("{:?}", tmpheader);

    //println!("{:?}", read_line(test));

    //let listener = TcpListener::bind("127.0.0.1:5555").unwrap();
    let listener = TcpListener::bind(combined).unwrap();

    // Spawn a thread for each connection
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let clonedpath = path.clone();
                thread::spawn(move|| {
                    // connection succeeded
                    let localpath = &clonedpath;
                    handle_client(stream, localpath);
                });
            }
            Err(e) => println!("A connection failed: {:?}", e),
        }
    }
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