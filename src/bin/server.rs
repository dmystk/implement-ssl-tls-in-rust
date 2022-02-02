use std::io::{Write, Read};
use std::cmp::Ordering;
use std::net::{TcpStream, TcpListener, SocketAddrV4, Ipv4Addr};

const PORT_KEY: &'static str = "PORT";
const DEFAULT_PORT: u16 = 7878;

fn main() {
    let port = std::env::var(PORT_KEY)
        .map(|port| { port.parse().unwrap() })
        .unwrap_or(DEFAULT_PORT);

    let listener = TcpListener::bind(
        SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port)
    ).unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                handle_connection(stream);
            },
            Err(e) => {
                eprintln!("Couldn't get client: {}", e);
            },
        }
    }
}

const MAX_CHUNK_SIZE: usize = 1024;

fn handle_connection(mut stream: TcpStream) {
    let mut buffer = [0; MAX_CHUNK_SIZE];

    stream.read(&mut buffer).unwrap();
    println!("{}", String::from_utf8_lossy(&buffer[..]));

    let response = match buffer[..4].cmp(b"GET ") {
        Ordering::Equal => {
            let contents = include_str!("../../res/index.html");
            format!("HTTP/1.1 200 OK\r\n\r\n{}", contents)
        },
        _ => {
            let contents = include_str!("../../res/501.html");
            format!("HTTP/1.1 501 NOT IMPLEMENTED\r\n\r\n{}", contents)
        },
    };

    stream.write(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}
