extern crate regex;

use std::io::prelude::*;
use std::env::args;
use std::process;
use std::fs::File;
use std::net::TcpStream;
use std::collections::HashMap;
use regex::Regex;



struct HttpHeaders {
    headers: HashMap<String, String>
}

impl HttpHeaders {
    fn add_str(&mut self, header: &str) {
        let split: Vec<&str> = header.splitn(2, ':').collect();
        let tag = split[0].trim().to_string();
        let value = split[1].trim().to_string();

        self.headers.insert(tag, value);
    }

    fn add(&mut self, tag: &str, value: &str) {
        self.headers.insert(tag.to_string(), value.to_string());
    }

    fn get(&self, tag: &str) -> &str{
        self.headers.get(tag).unwrap()
    }

    fn to_string(&self) -> String{
        let mut acum = String::new();
        for (tag, value) in &self.headers {
            acum = acum + &tag + ": " + &value + "\r\n";
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
        let splitted: Vec<&str> = message.splitn(2, "\r\n\r\n").collect();

        let mut headerlines = splitted[0].lines();
        let body = splitted[1].to_string();

        let init_line = (&headerlines.next().unwrap()).to_string();

        let mut headers = HttpHeaders::new();
        for line in headerlines {
            headers.add_str(line);
        }

        HttpMessage {init_line: init_line, headers: headers, body: body}
    }

    // Dpn't use this if the message is not a response
    fn status_code(&self) -> &str{
        self.init_line.split_whitespace().collect::<Vec<_>>()[1]
    }

    fn new() -> HttpMessage {
        HttpMessage {init_line: String::new(), headers: HttpHeaders::new(), body: String::new()}
    }
}



fn main() {

    let url;
    let dest;
    //let host;
    //let file;



    let args: Vec<String> = args().collect();

    if args.len() == 3 {
        url = args[1].clone();
        dest = args[2].clone();
    } else {
        println!("Usage:\n\t tool <url> </path/to/file>");
        process::exit(0);
    }

    let (host, file) = parse_uri(&url);

    let mut response = String::new();

    let mut connhost = host.to_string();

    loop {
        let mut stream = TcpStream::connect(&format!("{}:80", &connhost) as &str).unwrap();
        stream.write(request.to_string().as_bytes());
        stream.read_to_string(&mut response);
        hresponse = HttpMessage::from_str(&response);


        let status_code = hresponse.status_code();
        println!("Handling status code {}", status_code);

        if status_code == "301" {
            let url = hresponse.headers.get("Location");
            let (host, file) = parse_uri(url);
            request.init_line = format!("GET {} HTTP/1.1", file);
            request.headers.add("Host", host);
            connhost = host.to_string();
        }else if status_code == "200" {
            break;
        } else {
            println!("Unhandled status code {}", status_code);
            break;
        }

        response = String::new();        
    }

    



    let body = HttpMessage::from_str(&response).body;

    //println!("Response from server was {:?}", response);

    //println!("body is {}", body);

    let mut body_file = match File::create(&dest) {
        Err(_)      => panic!("Coudln't create file"),
        Ok(file)    => file,
    };

    match body_file.write_all(body.as_bytes()) {
        Err(_)      => panic!("Coudln't write to file"),
        Ok(_)       => println!("Copied contents from {} into {}", url, dest),
    };


}

fn parse_uri(uri: &str) -> (&str, &str) {

    // Url regex taken from RFC3986 https://tools.ietf.org/html/rfc3986#appendix-B
    let url_regex = Regex::new(r"^(([^:/?#]+):)?(//([^/?#]*))?([^?#]*)(\?([^#]*))?(#(.*))?").unwrap();

    let captured = url_regex.captures(uri).unwrap();

    println!("{}", uri);

    let host = captured.at(4).expect("Malformed url").trim();
    let file = captured.at(5).expect("Malformed url").trim();

    println!("host {} file {}", host, file);

    (host, file)
}
