#![feature(fs_time)] 
extern crate time;
extern crate clap;
extern crate regex;
extern crate chrono;
extern crate mime_guess;

// External libraries
use time::*;
use mime_guess::*;
use regex::Regex;
use chrono::{Local, FixedOffset};
use clap::{App, Arg};

// std libraries
use std::sync::Arc;
use std::thread;
use std::path::PathBuf;
use std::io::prelude::*;
use std::fs::File;
use std::fs;
use std::str;
use std::net::SocketAddr;
use std::net::{TcpListener, TcpStream};


/// Internal requesttypes
#[derive(Debug, PartialEq)]
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

/// Structure to allow the parsing of HTTP requests to be a lot less painful
#[derive(Debug)]
#[allow(non_snake_case)]
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
    /// Creates an empty HTTPHeader
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
    /// Parses the initial line of a request, and inserts it into the HTTPHeader struct
    /// Will return errors if the given line is in an incorrect format
    fn insert_init_line(&mut self, s: &'a str) -> Result<&'static str, &'static str> {

        let mut splitted = s.split_whitespace();


        let initline;

        // Checks that the request line contains a protocol, a path and a requesttype
        // splitted is an iterator so .nth() advances the iterator to the next step.
        // So this corrosponds to splitted[0] splitted[1] splitted[2]
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

        // Split the protocol into two for easier acccess
        match fullprot.find("/") {
            Some(i) => middle = i,
            None => return Result::Err("Invalid protocol format"),
        };


        let (prot, vers) = fullprot.split_at(middle);


        self.Protocol = Some(prot.trim());
        self.ProtocolVer = Some(vers[1..].trim());


        Result::Ok("OK")
    }

    /// inserts a header tag into the HTTPHeader struct. 
    /// Expects the line to contain a header, but will return an error if the header was malformed
    /// If the header is unsupport this returns a success but with the message "Unknown header"
    fn insert_tag(&mut self, s: &'a str) -> Result<&'static str, &'static str>{
        let middle;

        // Split the headers at ':'
        match s.find(":") {
            Some(i) => middle = i,
            None => return Result::Err("Invalid header format"),
        };

        let (header, tag) = s.split_at(middle);

        // Match the header with supported headers
        match header.trim() {
            "Connection" => self.Connection = Some(tag[1..].trim()),
            "Host" => self.Host = Some(tag[1..].trim()),
            "If-Modified-Since" => match date_from_str(tag[1..].trim()) {
                            Ok(t) => self.IfModifiedSince = Some(t),
                            Err(e) => return Result::Err(e),
                            },
            "If-Unmodified-Since" => match date_from_str(tag[1..].trim()) {
                            Ok(t) => self.IfUnmodifiedSince = Some(t),
                            Err(e) => return Result::Err(e),
                            },
            _ => return Result::Ok("Unknown header"),
        };


        Result::Ok("OK")
    }

    /// Fills up the HTTPHeader struct, by parsing a given buffer
    /// Will return an error if something was wrong with the format of the request
    fn parse_req(&mut self, req: &'a [u8]) -> Result<(),&'static str>{

        let mut len = 0;

        //Parse initialline
        match read_line(req) {
            Ok(s) => {
                match self.insert_init_line(s.trim()) {
                    Err(e) => return Result::Err(e),
                    _ => (),
                };

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

/// Attempts to parse the date written as either of these formats
/// Fri, 31 Dec 1999 23:59:59 GMT
/// Friday, 31-Dec-99 23:59:59 GMT
/// Fri Dec 31 23:59:59 1999
/// Will return an error if it's unable to parse the date
fn date_from_str(s: &str) -> Result<Tm, &'static str> {
    match   time::strptime(s, "%a, %d %b %Y %T %Z").or_else(|_| {
            time::strptime(s, "%A, %d-%b-%y %T %Z")}).or_else(|_| {
            time::strptime(s, "%c")}){
                Ok(t) => Ok(t),
                Err(_) => Err("Unable to parse date"),
                }
}

/// Converts a string into an internal RequestType for better error handling
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

/// Reads from the buf until it hits a newline.
/// Then reports back the slice until and containing the newline
/// Will return an error if buffer is an incorrect type
fn read_line(buf: &[u8]) -> Result<&str, Option<&str>> {
    let slice_res = str::from_utf8(buf);

    let slice;

    // Check if we can convert the buffer to a string
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

/// Called for each client connecting to the server
fn handle_client(mut stream: TcpStream, fileroot: &Arc<String>) {

    let ref derefed = **fileroot;
    let mut path = PathBuf::from(derefed);
    
    // 2k bytes should be enough for the headers
    let mut buffer = [0u8; 2048];


    let mut headers = HTTPHeader::new();

    // Pre initalize requesttype
    // Since otherwise if the request was horribly malformed we might now get a request
    // And then wouldn't know if we should send the body with or not
    let mut requesttype = RequestType::GET;


    // Read into the buffer
    let _ = stream.read(&mut buffer);


    // Try to parse the request, if it fails send the error message back
    match headers.parse_req(&buffer) {
        Ok(_) => {
            match headers.Type {
                Some(RequestType::HEAD) => requesttype = RequestType::HEAD,
                Some(RequestType::GET) => requesttype = RequestType::GET,
                Some(_) => {
                    send_error(stream, "501 Not Implemented", "The server does not support the method", requesttype);
                    return;
                },
                None => {
                    send_error(stream, "400 Bad Request", "Could not find method", requesttype);
                    return;
                },
            }
        },
        Err(e) => {
            send_error(stream, "400 Bad Request", e, requesttype);
            return;
        },
    };


    //Require host headers if they use version 1.1
    match headers.Host {
        None => match headers.ProtocolVer {
            Some("1.1") => {
                send_error(stream, "400 Bad Request", "Missing host header", requesttype);
                return;
            },
            _ => (),
        },
        _ => (),
    };


    // Doing some work, with the path given in the request
    // To see if it is valid
    if let Some(parsedpath) = headers.FilePath {

        //Spaces are replaced with %20 in requests. So we should reconvert it to spaces.
        let givenpath = parsedpath.to_string().replace("%20", " ");

        //Support requests from future versions of HTTP, that sends absolute paths
        match headers.ProtocolVer {
            Some("0.9") | Some("1.0") | Some("1.1") => path.push(&givenpath[1..]),
            _ => {
                let relpath;
                match from_absolute(&givenpath) {
                    Some(val) => relpath = val,
                    None => {
                        send_error(stream, "400 Bad Request", "Malformed absolute path", requesttype);
                        return;
                    },
                }
                path.push(&relpath[1..]);
            }
        };   
        
    } else {
        send_error(stream, "400 Bad Request", "Missing path", requesttype);
        return;
    }


    let metadata;
    let filehandle;

    // Try to open the requested file. Report back errors
    if let Ok(f) = File::open(&path) {
        match f.metadata() {
            Ok(m) => metadata = m,
            Err(_) => { 
                send_error(stream, "404 Not Found", "No such file on the server", requesttype);
                return;
            },
        };

        filehandle = f;
    } else {
        send_error(stream, "404 Not Found", "No such file on the server", requesttype);
        return;
    }

    // Handle the If-Modified-Since header
    if let Some(time) = headers.IfModifiedSince {
        let lastmod = metadata.modified().unwrap();

        let currenttime = time::get_time();
        let timesinceifmod; 
        // Dates in the future causes trouble
        match (currenttime - time.to_timespec()).to_std() {
            Ok(t) => timesinceifmod = t,
            Err(_) => {
                send_error(stream, "400 Bad Request", "Date is in the future", requesttype);
                return;
            },
        };
        let timesincemod = lastmod.elapsed().unwrap();

        if timesinceifmod < timesincemod {
            //Not modified since client last saw it
            send_error(stream, "304 Not Modified", "The requested file has not been modified", requesttype);
            return;
        }
    }

    // Handle the If-Unmodified-Since header
    if let Some(time) = headers.IfUnmodifiedSince {
        let lastmod = metadata.modified().unwrap();

        let currenttime = time::get_time();
        let timesinceifmod; 
        // Dates in the future causes trouble
        match (currenttime - time.to_timespec()).to_std() {
            Ok(t) => timesinceifmod = t,
            Err(_) => {
                send_error(stream, "400 Bad Request", "Date is in the future", requesttype);
                return;
            },
        };
        let timesincemod = lastmod.elapsed().unwrap();

        if timesinceifmod > timesincemod {
            //Modified since client last saw it
            send_error(stream, "412 Precondition Failed", "The requested file has been modified", requesttype);
            return;
        }

    }

    // Check if we should serve an index page or just a file
    if metadata.is_file() {
        send_file(stream, path, filehandle, requesttype);
    } else {
        send_index(stream, path, requesttype);
    }

    // TCP connection closes after this scope
}

/// Attempts to open the file, and sends it with the appropriate headers over a tcp stream
/// Will send errors if the file can't be opened
fn send_file(mut stream: TcpStream, path: PathBuf, mut file: File, req: RequestType) {
    let mut body: Vec<u8> = Vec::new();

    //Try to read the contents from the file into the body
    match file.read_to_end(&mut body) {
        Err(_) => { 
            send_error(stream, "404 Not Found", "No such file on the server", req);
            return;
        },
        _ => (),
    };

    // Use a utility to guess the type of the file.
    let mimetype = guess_mime_type(path);

    // Create the headers for the http response
    let msg = format!("HTTP/1.1 200 OK\r\n\
            Content-type: {}\r\n\
            Connection: close\r\n\
            Date: {}\r\n\
            Content-Length: {}\r\n\
            \r\n", mimetype, current_time(), body.len());

    // Write the headers over the stream
    let _ = stream.write_all(msg.as_bytes());

    // If it's a GET method also write the body
    if req == RequestType::GET {
        let _ = stream.write_all(&body);
    }    
}

fn send_index(mut stream: TcpStream, mut path: PathBuf, req: RequestType) {

    path.push("index.html");

    // Check if a index.html file exists
    match File::open(&path) {
        Ok(f) => {
            send_file(stream, path, f, req);
            return;
        }
        _ => (),
    }

    let mut indexpage = String::new();

    let dir = path.parent().unwrap();

    // Create an iterator over all of entrys in the folder
    let diriterator = match fs::read_dir(dir) {
        Ok(iter) => iter,
        Err(_) => {
            send_error(stream, "404 Not Found", "No such directory on the server", req);
            return;
        },
    };

    // Format the index kinda nicely
    indexpage.push_str(&format!("<html><body><h2>Index of {}/</h2><ul>",
                         dir.file_name().unwrap().to_str().unwrap()));

    // For each entry check if it can be opened, and then add them to the index page
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

    // Send the headers back
    let _ = stream.write_all(msg.as_bytes());

    // If the request was a get send the generated index page back
    if req == RequestType::GET {
        let _ = stream.write_all(indexpage.as_bytes());
    }
}

/// Extracts the reletive path out of a absolute path.
fn from_absolute(abspath: &str) -> Option<&str> {
    let url_regex = Regex::new(r"^(([^:/?#]+):)?(//([^/?#]*))?([^?#]*)(\?([^#]*))?(#(.*))?").unwrap();

    let captured = url_regex.captures(abspath).unwrap();

    captured.at(5)
}

/// Send an error message back over the TCP stream
fn send_error(mut stream: TcpStream, error_code: &str, reason: &str, req: RequestType) {

    let body = format!("<html>\r\n\
            <body>\r\n\
            <h2>Error {}</h2>\r\n\
            {}\r\n\
            </body>\r\n\
            </html>\r\n", error_code, reason);

    let msg = format!("HTTP/1.1 {}\r\n\
            Content-type: text-html\r\n\
            Connection: close\r\n\
            Date: {}\r\n\
            Content-Length: {}\r\n\
            \r\n", error_code, current_time(), body.len());

    let _ = stream.write_all(msg.as_bytes());

    if req == RequestType::GET {
        let _ = stream.write_all(body.as_bytes());
    }
}

/// Get current time in GMT, and format it to be standards compliant
fn current_time() -> String {
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

    // Set the adress to the given adress, or to 0.0.0.0 if none was given
    let address = arguments.value_of("adress").unwrap_or("0.0.0.0");
    let port = arguments.value_of("port").unwrap();
    let serverroot = arguments.value_of("path").unwrap();

    let path = Arc::new(serverroot.to_string());

    // Check if the adress combined with the port corrosponds to a valid adress we can listen on
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

/* TESTING */


#[test]
fn test_parsedates() {
    // Try to parse each of the three possible formats, will panic if either returns an error
    date_from_str("Fri, 31 Dec 1999 23:59:59 GMT").unwrap();
    date_from_str("Friday, 31-Dec-99 23:59:59 GMT").unwrap();
    date_from_str("Fri Dec 31 23:59:59 1999").unwrap();
}

#[test]
fn test_abs_path() {
    // Make sure that the absolute path can be parsed correctly
    assert_eq!(from_absolute("http://www.somehost.com/path/file.html").unwrap(), "/path/file.html");
    assert_eq!(from_absolute("http://www.somehost.com:80/path/file.html").unwrap(), "/path/file.html");
}


#[test]
fn test_read_line() {
    // Tests for read line

    let linedmsg = "foo\r\nbar\r\n";

    // First line should be foo
    let read = read_line(linedmsg.as_bytes()).unwrap();
    assert_eq!(read, "foo\r\n");

    // Second line should be bar
    let readtwo = read_line(linedmsg[read.len()..].as_bytes()).unwrap();
    assert_eq!(readtwo, "bar\r\n");
}


#[test]
#[should_panic]
fn test_read_line_fail() {
    // Test to make sure it fails when EOF

    let linedmsg = "";

    // This should fail since the unwrap will panic as it tries to unwrap an error
    let read = read_line(linedmsg.as_bytes()).unwrap();
    assert_eq!(read, "");
}

#[test]
fn test_parse_req_normal_get() {
    // Test to make sure we can parse a normal request

    let req = "GET /path.ending HTTP/1.1\r\n\
    User-Agent: Mozilla/4.0 (compatible; MSIE5.01; Windows NT)\r\n\
    Host: localhost\r\n\
    Accept-Language: en-us\r\n\
    Accept-Encoding: gzip, deflate\r\n\
    Connection: Keep-Alive\r\n\
    \r\n".as_bytes();

    let mut header = HTTPHeader::new();


    let parse_result = header.parse_req(req);

    // Check if the result was ok
    parse_result.unwrap();

    // Check to see if our supported headers got added as we wanted
    assert_eq!(header.Type, Some(RequestType::GET));
    assert_eq!(header.Connection, Some("Keep-Alive"));
    assert_eq!(header.Protocol, Some("HTTP"));
    assert_eq!(header.ProtocolVer, Some("1.1"));
    assert_eq!(header.FilePath, Some("/path.ending"));
}

#[test]
fn test_parse_req_normal_head() {
    // Test to make sure we can parse a normal request

    let req = "HEAD /path.ending HTTP/1.1\r\n\
    User-Agent: Mozilla/4.0 (compatible; MSIE5.01; Windows NT)\r\n\
    Host: localhost\r\n\
    Accept-Language: en-us\r\n\
    Accept-Encoding: gzip, deflate\r\n\
    Connection: Keep-Alive\r\n\
    \r\n".as_bytes();

    let mut header = HTTPHeader::new();


    let parse_result = header.parse_req(req);

    // Check if the result was ok
    parse_result.unwrap();

    // Check to see if our supported headers got added as we wanted
    assert_eq!(header.Type, Some(RequestType::HEAD));
    assert_eq!(header.Connection, Some("Keep-Alive"));
    assert_eq!(header.Protocol, Some("HTTP"));
    assert_eq!(header.ProtocolVer, Some("1.1"));
    assert_eq!(header.FilePath, Some("/path.ending"));
}

#[test]
#[should_panic]
fn test_parse_req_missing_blankline() {
    // Oh no they forgot the last blankline
    // We should report an error

    let req = "HEAD /path.ending HTTP/1.1\r\n\
    User-Agent: Mozilla/4.0 (compatible; MSIE5.01; Windows NT)\r\n\
    Host: localhost\r\n\
    Accept-Language: en-us\r\n\
    Accept-Encoding: gzip, deflate\r\n\
    Connection: Keep-Alive\r\n".as_bytes();

    let mut header = HTTPHeader::new();


    let parse_result = header.parse_req(req);

    // Check if the result was ok
    parse_result.unwrap();

    // Check to see if our supported headers got added as we wanted
    assert_eq!(header.Type, Some(RequestType::HEAD));
    assert_eq!(header.Connection, Some("Keep-Alive"));
    assert_eq!(header.Protocol, Some("HTTP"));
    assert_eq!(header.ProtocolVer, Some("1.1"));
    assert_eq!(header.FilePath, Some("/path.ending"));
}


#[test]
#[should_panic]
fn test_parse_req_missing_type() {
    // Oh no they forgot to set the request type
    // We should report an error

    let req = "/path.ending HTTP/1.1\r\n\
    User-Agent: Mozilla/4.0 (compatible; MSIE5.01; Windows NT)\r\n\
    Host: localhost\r\n\
    Accept-Language: en-us\r\n\
    Accept-Encoding: gzip, deflate\r\n\
    Connection: Keep-Alive\r\n\
    \r\n".as_bytes();

    let mut header = HTTPHeader::new();


    let parse_result = header.parse_req(req);

    // Check if the result was ok
    parse_result.unwrap();

    // Check to see if our supported headers got added as we wanted
    assert_eq!(header.Type, Some(RequestType::HEAD));
    assert_eq!(header.Connection, Some("Keep-Alive"));
    assert_eq!(header.Protocol, Some("HTTP"));
    assert_eq!(header.ProtocolVer, Some("1.1"));
    assert_eq!(header.FilePath, Some("/path.ending"));
}

#[test]
#[should_panic]
fn test_parse_req_missing_protocol() {
    // Oh no they forgot to set the protocol
    // We should report an error

    let req = "GET /path.ending\r\n\
    User-Agent: Mozilla/4.0 (compatible; MSIE5.01; Windows NT)\r\n\
    Host: localhost\r\n\
    Accept-Language: en-us\r\n\
    Accept-Encoding: gzip, deflate\r\n\
    Connection: Keep-Alive\r\n\
    \r\n".as_bytes();

    let mut header = HTTPHeader::new();


    let parse_result = header.parse_req(req);

    // Check if the result was ok
    parse_result.unwrap();

    // Check to see if our supported headers got added as we wanted
    assert_eq!(header.Type, Some(RequestType::HEAD));
    assert_eq!(header.Connection, Some("Keep-Alive"));
    assert_eq!(header.Protocol, Some("HTTP"));
    assert_eq!(header.ProtocolVer, Some("1.1"));
    assert_eq!(header.FilePath, Some("/path.ending"));
}

#[test]
#[should_panic]
fn test_parse_req_missing_filepath() {
    // Oh no they forgot to set the filepath
    // We should report an error

    let req = "GET HTTP/1.1\r\n\
    User-Agent: Mozilla/4.0 (compatible; MSIE5.01; Windows NT)\r\n\
    Host: localhost\r\n\
    Accept-Language: en-us\r\n\
    Accept-Encoding: gzip, deflate\r\n\
    Connection: Keep-Alive\r\n\
    \r\n".as_bytes();

    let mut header = HTTPHeader::new();


    let parse_result = header.parse_req(req);

    // Check if the result was ok
    parse_result.unwrap();

    // Check to see if our supported headers got added as we wanted
    assert_eq!(header.Type, Some(RequestType::HEAD));
    assert_eq!(header.Connection, Some("Keep-Alive"));
    assert_eq!(header.Protocol, Some("HTTP"));
    assert_eq!(header.ProtocolVer, Some("1.1"));
    assert_eq!(header.FilePath, Some("/path.ending"));
}