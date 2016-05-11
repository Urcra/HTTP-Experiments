extern crate regex;

use std::io::prelude::*;
use std::env::args;
use std::process;
use std::net::TcpStream;
use regex::Regex;


fn main() {

    let url;
    let dest;
    let host;
    let file;



    let args: Vec<String> = args().collect();

    if args.len() == 3 {
        url = args[1].clone();
        dest = args[2].clone();
    } else {
        println!("Usage:\n\t tool <url> </path/to/file>");
        process::exit(0);
    }

    // Url regex taken from RFC3986 https://tools.ietf.org/html/rfc3986#appendix-B
    let url_regex = Regex::new(r"^(([^:/?#]+):)?(//([^/?#]*))?([^?#]*)(\?([^#]*))?(#(.*))?").unwrap();

    let captured = url_regex.captures(&url).unwrap();

    host = captured.at(4).expect("Malformed url");
    file = match captured.at(5) {
        Some("") => "/",
        Some(x)  => x,
        _        => "/",
    };

    let mut stream = TcpStream::connect(&format!("{}:80", host) as &str).unwrap();


    let message = format!("GET {} HTTP/1.1\r\n\
                        Host: {}\r\n\
                        Connection: close\r\n\
                        From: user@user.com\r\n\
                        User-Agent: badHTTP/0.1\r\n\
                        \r\n", file, host);

    let _ = stream.write(message.as_bytes());

    let mut response = String::new(); 

    let _ = stream.read_to_string(&mut response);

    println!("Response from server was {:?}", response);


}