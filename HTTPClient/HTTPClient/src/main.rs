extern crate regex;

use std::io::prelude::*;
use std::env::args;
use std::process;
use std::net::TcpStream;
use std::collections::HashMap;
use regex::Regex;



struct HttpHeaders {
    headers: HashMap<String, String>
}

impl HttpHeaders {
    fn add(&mut self, header: &str) {
        let split: Vec<&str> = header.split(':').collect();
        let tag = split[0].trim().to_string();
        let value = split[1].trim().to_string();

        self.headers.insert(tag, value);
    }

    fn get(&self, tag: &str) -> &str{
        self.headers.get(tag).unwrap()
    }

    fn to_string(&self) -> String{
        let mut acum = String::new();
        for (tag, value) in &self.headers {
            acum = acum + &tag + " : " + &value + "\r\n";
        }
        acum
    }

    fn new() -> HttpHeaders {
        HttpHeaders{headers: HashMap::new()}
    }
}


struct HttpMessage {
    init_line: String,
    headers: HttpHeaders,
    body: String,
}

impl HttpMessage {
    fn to_string(&self) -> String {
        String::new() + &self.init_line + "\r\n" + &self.headers.to_string() + "\r\n" + &self.body
    }

    fn from_str(message: &str) -> HttpMessage {
        let mut lines = message.lines();
        let init_line = (&lines.next().unwrap()).to_string();
        let mut headers = HttpHeaders::new();

        // TODO make this better

        loop {
            let line = lines.next();
            match line {
                Some("") => break,
                Some(x) => headers.add(x),
                None    => break,
            };
        }

        let mut body = String::new();

        loop {
            let line = lines.next();
            match line {
                Some(x) => body = body + x + "\r\n",
                None    => break,
            };
        }


        HttpMessage {init_line: init_line, headers: headers, body: body}
    }
}



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

    let body = HttpMessage::from_str(&response).body;

    println!("Response from server was {:?}", response);

    println!("body is {}", body);


}