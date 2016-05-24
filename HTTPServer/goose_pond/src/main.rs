#![recursion_limit="500"]

extern crate time;
#[macro_use] extern crate mioco;
extern crate mio;


use mio::*;
use mio::tcp::*;
use std::collections::HashMap;

use std::fs::File;

use time::*;
use std::str;
use std::net::SocketAddr;
use std::str::FromStr;
use std::io::{self, Write, Read};


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
struct HTTPHeader {
    //Iinital line
    Protocol: Option<String>,
    ProtocolVer: Option<String>,
    FilePath: Option<String>,
    Type: Option<RequestType>,
    //Header tags
    Connection: Option<String>,
    Host: Option<String>,
    IfModifiedSince: Option<Tm>,
    IfUnmodifiedSince: Option<Tm>,
}

impl HTTPHeader {
    fn new() -> HTTPHeader{
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
    fn insert_init_line(&mut self, s: &str) -> Result<&'static str, &'static str> {

        let mut splitted = s.split_whitespace();


        let initline;


        match (splitted.nth(0), splitted.nth(0), splitted.nth(0)) {
            (None, _, _) => return Result::Err("Invalid init line"),
            (_, None, _) => return Result::Err("Invalid init line"),
            (_, _, None) => return Result::Err("Invalid init line"),
            (Some(x), Some(y), Some(z)) => initline = (x.trim(), y.trim(), z.trim()),
        };


        let (reqtype, path, fullprot) = initline;


        self.FilePath = Some(path.to_string());
        self.Type = Some(reqtype_from_str(reqtype));


        let middle;

        match fullprot.find("/") {
            Some(i) => middle = i,
            None => return Result::Err("Invalid header format"),
        };


        let (prot, vers) = fullprot.split_at(middle);


        self.Protocol = Some(prot.trim().to_string());
        self.ProtocolVer = Some((vers[1..].trim()).to_string());


        Result::Ok("OK")
    }

    fn insert_tag(&mut self, s: &str) -> Result<&'static str, &'static str>{
        let middle;

        match s.find(":") {
            Some(i) => middle = i,
            None => return Result::Err("Invalid header format"),
        };

        let (header, tag) = s.split_at(middle);


        match header.trim() {
            "Connection" => self.Connection = Some((tag[1..].trim()).to_string()),
            "Host" => self.Host = Some((tag[1..].trim()).to_string()),
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

    fn parse_req(&mut self, req: &[u8]) -> Result<(),()>{

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










struct HTTPConn {
    socket: TcpStream,
    headers: HTTPHeader,
    interest: EventSet,
}

impl HTTPConn {
    fn new(socket: TcpStream) -> HTTPConn {
        let headers = HTTPHeader::new();

        HTTPConn {
            socket: socket,
            headers: headers,
            interest: EventSet::readable(),
        }
    }

    fn write(&mut self) {
        //let headers = self.headers;


        match self.headers.FilePath {
            Some(ref f) => {self.socket.try_write(RESPONSE.as_bytes()).unwrap();},
            None    => {println!("heyo");},
        };
        /*let response = fmt::format(format_args!("HTTP/1.1 101 Switching Protocols\r\n\
                                                 Connection: Upgrade\r\n\
                                                 Sec-WebSocket-Accept: {}\r\n\
                                                 Upgrade: websocket\r\n\r\n", response_key));*/
        self.socket.try_write(RESPONSE.as_bytes()).unwrap();

        self.interest.remove(EventSet::writable());
        //self.interest.insert(EventSet::readable());
    }

    fn read(&mut self) {
        loop {
            let mut buf = [0; 2048];
            match self.socket.try_read(&mut buf) {
                Err(e) => {
                    println!("Error while reading socket: {:?}", e);
                    return
                },
                Ok(None) =>
                    // Socket buffer has got no more bytes.
                    break,
                Ok(Some(len)) => {
                    //println!("readsocket");
                    //self.http_parser.parse(&buf);
                    let res = self.headers.parse_req(&buf);

                    if res == Ok(()) {
                        // Change the current state
                        //self.state = ClientState::HandshakeResponse;

                        // Change current interest to `Writable`
                        self.interest.remove(EventSet::readable());
                        self.interest.insert(EventSet::writable());
                        break;
                    }
                }
            }
        }
    }
}

struct WebSocketServer {
    socket: TcpListener,
    clients: HashMap<Token, HTTPConn>,
    token_counter: usize
}

const SERVER_TOKEN: Token = Token(0);

impl Handler for WebSocketServer {
    type Timeout = usize;
    type Message = ();

    fn ready(&mut self, event_loop: &mut EventLoop<WebSocketServer>, token: Token, events: EventSet) {
        if events.is_readable() {
            match token {
                SERVER_TOKEN => {
                    let client_socket = match self.socket.accept() {
                        Ok(Some((sock, addr))) => sock,
                        Ok(None) => unreachable!(),
                        Err(e) => {
                            println!("Accept error: {}", e);
                            return;
                        }
                    };

                    let new_token = Token(self.token_counter);
                    self.clients.insert(new_token, HTTPConn::new(client_socket));
                    self.token_counter += 1;

                    event_loop.register(&self.clients[&new_token].socket, new_token, EventSet::readable(),
                                        PollOpt::edge() | PollOpt::oneshot()).unwrap();
                },
            token => {
                    let mut client = self.clients.get_mut(&token).unwrap();
                    client.read();
                    event_loop.reregister(&client.socket, token, client.interest,
                                          PollOpt::edge() | PollOpt::oneshot()).unwrap();
                }
            }
        }

        if events.is_writable() {
            let mut client = self.clients.get_mut(&token).unwrap();
            client.write();
            event_loop.reregister(&client.socket, token, client.interest,
                                  PollOpt::edge() | PollOpt::oneshot()).unwrap();
        }
    }
}






fn main() {
    //println!("Hello, world!");


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

    //let addr: SocketAddr = FromStr::from_str("127.0.0.1:5555").unwrap();

    //let listener = TcpListener::bind(&addr).unwrap();





    let address = "127.0.0.1:5555".parse::<SocketAddr>().unwrap();
    let server_socket = TcpListener::bind(&address).unwrap();

    let mut event_loop = EventLoop::new().unwrap();

    let mut server = WebSocketServer {
        token_counter: 1,
        clients: HashMap::new(),
        socket: server_socket
    };

    event_loop.register(&server.socket,
                        SERVER_TOKEN,
                        EventSet::readable(),
                        PollOpt::edge()).unwrap();
    event_loop.run(&mut server).unwrap();





    /*


    // Supporting persistent connections woo
    mioco::start(move || {
        for _ in 0..mioco::thread_num() {
            let listener = listener.try_clone().unwrap();
            mioco::spawn(move || {
                loop {
                    let mut conn = listener.accept().unwrap();
                    mioco::spawn(move || -> io::Result<()> {
                        let mut buf_i = 0;
                        let mut buf = [0u8; 1024];
                        let mut foo = Vec::new();

                        let mut msg: Vec<u8> = Vec::new();

                        msg.extend_from_slice(RESPONSE.as_bytes());
                        

                        


                            loop {
                                let mut headers = HTTPHeader::new();

                                let len = try!(conn.read(&mut buf[buf_i..]));

                                if len == 0 {
                                    // Spurrious event can fire, so check if we actually receive something.
                                    return Ok(());
                                }

                                buf_i += len;

                                let res = headers.parse_req(&buf[0..buf_i]);

                                if res == Ok(()) {
                                    match headers.FilePath {
                                        Some(path) => {

                                            mioco::sync( || -> io::Result<()> {
                                                println!("here1");
                                            let mut f = try!(File::open("foo.txt"));
                                            println!("here2");
                                            println!("here3");
                                            try!(f.read_to_end(&mut foo));
                                            //println!("{:?}", foo);
                                            println!("here4");

                                            
                                            return Ok(());
                                            }
                                            );
                                            println!("writing foo");

                                            msg.append(&mut foo);

                                            try!(conn.write_all(&msg));

                                            //try!(conn.write_all(&foo));

                                            //try!(conn.write_all(&RESPONSE_NULL.as_bytes()));

                                            //try!(conn.write_all(&RESPONSE.as_bytes()));
                                            
                                            buf_i = 0;
                                            return Ok(());
                                        },                                               
                                        None       => {
                                            try!(conn.write_all(&RESPONSE.as_bytes()));
                                            buf_i = 0;
                                            return Ok(());
                                        },
                                    }
                                }
                            }
                        


                        /*
                        loop {
                            let mut headers = HTTPHeader::new();

                            let len = try!(conn.read(&mut buf[buf_i..]));

                            if len == 0 {
                                return Ok(());
                            }

                            buf_i += len;

                            let res = headers.parse_req(&buf[0..buf_i]);

                            if res == Ok(()) {
                                //println!("{:?}", headers);
                                match headers.FilePath {
                                    Some(path) => {
                                        try!(conn.write_all(&RESPONSE.as_bytes()));
                                        buf_i = 0;
                                    },
                                    None       => {
                                        try!(conn.write_all(&RESPONSE.as_bytes()));
                                        return Ok(());
                                    },
                                }
                            }
                        }*/

                    });
                }
            });
        }
    }).unwrap();


    */
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