extern crate regex;

use std::io::prelude::*;
use std::fs::File;
use std::io::BufReader;
use std::io::BufWriter;
use std::env::args;
use std::process;
use std::str;
use std::collections::HashMap;
use std::net::TcpStream;
use regex::Regex;



fn main() {


    let url = args().nth(1).expect("Usage:\n\t tool <url> [/path/to/file]");

    let mut output: BufWriter<Box<Write>> = BufWriter::new(
        if let Some(f) = args().nth(2) {
            Box::new(File::create(f).expect("Could not create file"))
        } else {
            Box::new(std::io::stdout())
        });



    let (host, file) = parse_uri(&url);

    let stream = send_request(host, file);

    let (init_line, mut headers, stream) = parse_response_head(stream);


    let mut buffer = String::new();
    let mut reader = BufReader::new(&stream);



    if let Some(encoding) = headers.get("Transfer-Encoding") {
        if encoding == "chunked" {
            loop {
                reader.read_line(&mut buffer);
                println!("{:?}", buffer);
                buffer = buffer.trim().to_string();
                if buffer == "0"{
                    buffer.clear();
                    break;
                }
                let content_length: u64 = u64::from_str_radix(&buffer, 16).unwrap();

                let mut chunk = (&mut reader).take(content_length + 2);
                let mut outchunk: Vec<u8> = Vec::new(); 
                chunk.read_to_end(&mut outchunk);
                output.write_all(&outchunk[0..outchunk.len()-2]);
                //chunk.read_to_end(&mut body);

                buffer.clear();
            }
        }
    } else {
        let length = headers.get("Content-Length").unwrap();
        let content_length: u64 = u64::from_str_radix(&length, 10).unwrap();

        let mut chunk = (&mut reader).take(content_length);
        let mut outchunk: Vec<u8> = Vec::new(); 
        chunk.read_to_end(&mut outchunk);
        output.write_all(&outchunk);
    }


    println!("Init line was  {:?}", init_line);
    println!("Headers were {:?}", headers);



}

fn send_request<'a>(host: &str, file: &str) -> TcpStream {
    let mut stream = TcpStream::connect(&format!("{}:80", host) as &str).unwrap();

    let message = format!("GET {} HTTP/1.1\r\n\
                        Host: {}\r\n\
                        Connection: close\r\n\
                        User-Agent: SillyGoose/0.1\r\n\
                        \r\n", file, host);

    let _ = stream.write(message.as_bytes());

    stream
}

fn parse_response_head(stream: TcpStream) -> (String, HashMap<String, String>, TcpStream) {
    let mut buffer = String::new();
    let mut init_line = String::new();
    let mut headers = HashMap::new();
    // More fun lifetimes hacks
    {
        let mut reader = BufReader::new(&stream);

        let _ = reader.read_line(&mut init_line);

        loop {
            reader.read_line(&mut buffer);
            if buffer == "\r\n" {
                buffer.clear();
                break;
            } else {
                // Not an unnecessary scope, needed for the borrow checker to not complain
                {
                    let split: Vec<&str> = buffer.splitn(2, ':').collect();
                    let tag = split[0].trim().to_string();
                    let value = split[1].trim().to_string();

                    headers.insert(tag, value);
                }
                buffer.clear();
            }
        }
    }
    (init_line, headers, stream)
}

fn parse_uri(uri: &str) -> (&str, &str) {
    // Url regex taken from RFC3986 https://tools.ietf.org/html/rfc3986#appendix-B
    let url_regex = Regex::new(r"^(([^:/?#]+):)?(//([^/?#]*))?([^?#]*)(\?([^#]*))?(#(.*))?").unwrap();

    let captured = url_regex.captures(uri).unwrap();

    let host = captured.at(4).expect("Malformed url");
    let file = captured.at(5).expect("Malformed url");

    (host, file)
}