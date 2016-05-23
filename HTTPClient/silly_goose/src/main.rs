extern crate regex;

use std::io::prelude::*;
use std::fs::File;
use std::io::BufReader;
use std::io::BufWriter;
use std::env::args;
use std::collections::HashMap;
use std::net::TcpStream;
use regex::Regex;



fn main() {


    // Extract command line arguments
    let mut url = args().nth(1).expect("Usage:\n\t silly_goose <url> [/path/to/file]");

    // Sets the output to either a file or stdout, depending on if a second argument exists
    let output: BufWriter<Box<Write>> = BufWriter::new(
        if let Some(f) = args().nth(2) {
            Box::new(File::create(f).expect("Could not create file"))
        } else {
            Box::new(std::io::stdout())
        });


    // Keep looping until we don't get redirected anymore
    loop {

        let tmpurl = url.to_owned();
        let (host, file) = parse_uri(&tmpurl);


        let stream = send_request(host, file);

        let (init_line, headers, reader) = parse_response_head(stream);


        let status_code = init_line.split_whitespace().collect::<Vec<_>>()[1];

        //Code 100 is handled in the parsing.
        if status_code == "200" {
            write_body(headers, reader, output);
            break;
        } else if status_code == "300" || status_code == "301"
                 || status_code == "302" || status_code == "303" {

        //Extract location header, and try again
            if let Some(loc) = headers.get("Location") {
                println!("Status code {}. Retrieving content from {}",status_code, loc);
                url = loc.to_owned();
            }

        } else {
            panic!("Sever returned unexpected code: {}", status_code);
        }
    }


}

fn write_body(headers: HashMap<String, String>, mut reader: BufReader<TcpStream>, mut output: BufWriter<Box<Write>>) {


    let mut buffer = String::new();

    if let Some(encoding) = headers.get("Transfer-Encoding") {
        if encoding == "chunked" {
            loop {
                let _ = reader.read_line(&mut buffer);
                buffer = buffer.trim().to_string();
                if buffer == "0"{
                    buffer.clear();
                    break;
                }
                let content_length: u64 = u64::from_str_radix(&buffer, 16).unwrap();

                let mut chunk = (&mut reader).take(content_length + 2);
                let mut outchunk: Vec<u8> = Vec::new(); 
                let _ = chunk.read_to_end(&mut outchunk);
                let _ = output.write_all(&outchunk[0..outchunk.len()-2]);

                buffer.clear();
            }
        } else {
            panic!("Currently no support for decoding {}", encoding);
        }
    } else {
        let length = headers.get("Content-Length").unwrap();
        let content_length: u64 = u64::from_str_radix(&length, 10).unwrap();

        let mut chunk = (&mut reader).take(content_length);
        let mut outchunk: Vec<u8> = Vec::new(); 
        let _ = chunk.read_to_end(&mut outchunk);
        let _ = output.write_all(&outchunk);
    }



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

fn parse_response_head(stream: TcpStream) -> (String, HashMap<String, String>, BufReader<TcpStream>) {
    let mut buffer = String::new();
    let mut init_line = String::new();
    let mut headers = HashMap::new();   
    let mut reader = BufReader::new(stream);

    let _ = reader.read_line(&mut init_line);
    
    let tmp = init_line.to_owned();

    let status_code = tmp.split_whitespace().collect::<Vec<_>>()[1];

    //Ignore code 100, and read past

    if status_code == "100" {
        init_line.clear();
        let _ = reader.read_line(&mut init_line);
        let _ = reader.read_line(&mut buffer);
        buffer.clear();

    }
    
    loop {
        let _ = reader.read_line(&mut buffer);
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
    
    (init_line, headers, reader)
}

fn parse_uri(uri: &str) -> (&str, &str) {
    // Url regex taken from RFC3986 https://tools.ietf.org/html/rfc3986#appendix-B
    let url_regex = Regex::new(r"^(([^:/?#]+):)?(//([^/?#]*))?([^?#]*)(\?([^#]*))?(#(.*))?").unwrap();

    let captured = url_regex.captures(uri).unwrap();

    let host = captured.at(4).expect("Malformed url");
    let file = captured.at(5).expect("Malformed url");

    (host, file)
}