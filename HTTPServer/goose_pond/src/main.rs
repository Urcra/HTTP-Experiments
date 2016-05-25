#![feature(fs_time)] 
extern crate time;
extern crate clap;
extern crate regex;
extern crate chrono;
extern crate mime_guess;

use std::io::prelude::*;
use std::sync::Arc;
use std::path::PathBuf;
use std::path::Path;
use std::fs::File;
use std::fs;
use clap::{App, Arg, SubCommand};
use time::*;
use mime_guess::*;
use chrono::{Local, UTC, FixedOffset};
use std::str;
use std::net::SocketAddr;
use std::str::FromStr;
//use std::io::{self, Write, Read};
use std::net::{TcpListener, TcpStream};
use std::thread;
use regex::Regex;
//use mioco::tcp::TcpListener;




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


fn handle_client(mut stream: TcpStream, fileroot: &Arc<String>) {

    let ref derefed = **fileroot;
    let mut path = PathBuf::from(derefed);


    let mut buffer = [0u8; 2048];
    //let mut foo = Vec::new();
    
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


    if let Some(parsedpath) = headers.FilePath {

        //Spaces are replaced with %20 in requests. So we should reconvert it to spaces.
        let mut givenpath = parsedpath.to_string().replace("%20", " ");

        //Support requests from future versions of HTTP, that sends absolute paths
        match headers.ProtocolVer {
            Some("0.9") | Some("1.0") | Some("1.1") => path.push(&givenpath[1..]),
            _ => {
                let relpath;
                match from_absolute(&givenpath) {
                    Some(val) => relpath = val,
                    None => {
                        send_error(stream, "400 Bad Request", "Malformed absolute path");
                        return;
                    },
                }
                path.push(&relpath[1..]);
            }
        };   
        
    } else {
        send_error(stream, "400 Bad Request", "Missing path");
        return;
    }


    let metadata;
    let mut filehandle;

    if let Ok(mut f) = File::open(&path) {
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
        println!("{:?}", &path);
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
        send_file(stream, path, filehandle);
    } else {
        send_index(stream, path);
    }

    //TODO add dates to header
    //TODO handle index and files

    // TCP connection closes after this scope
}

fn send_file(mut stream: TcpStream, path: PathBuf, mut file: File) {
    let mut body: Vec<u8> = Vec::new();

    match file.read_to_end(&mut body) {
        Err(_) => { 
            send_error(stream, "404 Not Found", "No such file on the server");
            return;
        },
        _ => (),
    };

    let mimetype = guess_mime_type(path);

    let msg = format!("HTTP/1.1 200 OK\r\n\
            Content-type: {}\r\n\
            Connection: close\r\n\
            Date: {}\r\n\
            Content-Length: {}\r\n\
            \r\n", mimetype, current_time(), body.len());

    stream.write_all(msg.as_bytes());
    stream.write_all(&body).unwrap();
}

fn send_index(mut stream: TcpStream, mut path: PathBuf) {

    path.push("index.html");

    match File::open(&path) {
        Ok(f) => {
            send_file(stream, path, f);
            return;
        }
        _ => (),
    }

    let mut indexpage = String::new();

    let dir = path.parent().unwrap();

    let diriterator = match fs::read_dir(dir) {
        Ok(iter) => iter,
        Err(_) => {
            send_error(stream, "404 Not Found", "No such directory on the server");
            return;
        },
    };

    indexpage.push_str(&format!("<html><body><h2>Index of {}/</h2><ul>",
                         dir.file_name().unwrap().to_str().unwrap()));

    for maybeentry in diriterator {
        let entry;
        match maybeentry {
            Ok(o) => entry = o,
            Err(_) => continue,
        };
        let filetype = entry.file_type().unwrap();

        if filetype.is_dir() {
            let entry_name = entry.file_name().into_string().unwrap();
            let fname = format!("<li><a href=\"{}/\">{}/</a></li>", entry_name, entry_name);
            indexpage.push_str(&fname);
        } else if filetype.is_file() {
            let entry_name = entry.file_name().into_string().unwrap();
            let fname = format!("<li><a href=\"{}\">{}</a></li>", entry_name, entry_name);
            indexpage.push_str(&fname);
        }

    }

    indexpage.push_str("</ul></body></html>");

    let msg = format!("HTTP/1.1 200 OK\r\n\
            Content-type: text-html\r\n\
            Connection: close\r\n\
            Date: {}\r\n\
            Content-Length: {}\r\n\
            \r\n", current_time(), indexpage.as_bytes().len());

    stream.write_all(msg.as_bytes()).unwrap();
    stream.write_all(indexpage.as_bytes()).unwrap();

    //unimplemented!();
}

fn from_absolute(abspath: &str) -> Option<&str> {
    let url_regex = Regex::new(r"^(([^:/?#]+):)?(//([^/?#]*))?([^?#]*)(\?([^#]*))?(#(.*))?").unwrap();

    let captured = url_regex.captures(abspath).unwrap();

    //fix crashing
    captured.at(5)
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
            Date: {}\r\n\
            Content-Length: {}\r\n\
            \r\n\
            {}", error_code, current_time(), body.len(), body);

    stream.write_all(msg.as_bytes());
}

fn current_time() -> String {
    // Get current time in GMT, and format it to be standards compliant
    Local::now().with_timezone(&FixedOffset::east(0)).to_rfc2822().replace("+0000", "GMT")
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