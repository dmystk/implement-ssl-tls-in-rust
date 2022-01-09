
use std::io::{Write, Read};  // need to use stream.read() and stream.write()
use std::net::TcpStream;
use url::Url;
use structopt::StructOpt;

/// Struct for CLI arguments.
#[derive(Debug, StructOpt)]
#[structopt(name = "client", about = "Simple CLI HTTP client.")]
struct Opt {
    #[structopt(name = "URL")]
    pub url: Url,
    #[structopt(short, long, help = "Set proxy server URL")]
    pub proxy: Option<Url>,
}

/// Exit program with printing a message to stderr.
macro_rules! exit_with_message {
    ($($arg:tt)*) => {
        eprintln!($($arg)*);
        std::process::exit(-1);
    }
}

fn main() {
    // check command line arguments
    let opt = Opt::from_args();
    let url = opt.url;
    if url.scheme() != "http" {
        exit_with_message!("Unsupported shceme: {}", url.scheme());
    }
    let proxy = opt.proxy;
    if proxy.is_some() && proxy.as_ref().unwrap().scheme() != "http" {
        exit_with_message!("Unsupported proxy shceme: {}", proxy.unwrap().scheme());
    }

    // request HTTP GET
    let print_bytes = |bytes: &[u8]| {
        print!("{}", std::str::from_utf8(bytes).unwrap());
    };
    if proxy.is_some() {
        let proxy_url = proxy.unwrap();
        request_http_get_with_proxy(&url, &proxy_url, print_bytes).unwrap_or_else(|e| {
            exit_with_message!("{}", e);
        });
    } else {
        request_http_get(&url, print_bytes).unwrap_or_else(|e| {
            exit_with_message!("{}", e);
        });
    }
}

/// Unwrap Ok value or terminate function with Err as return value
/// i.e. this macro is only for functions that have Result(..) as return type.
/// This macro is to avoid deep nesting by `match` expression.
macro_rules! unwrap_or_return_err {
    ($e:expr) => {
        match $e {
            Ok(x) => x,
            Err(e) => return Err(e),
        }
    };
}

// Request HTTP GET and process response stream with the callback function.
// The callback is called chunk by chunk.
fn request_http_get(url: &Url, callback: fn(&[u8])) -> std::io::Result<()> {
    // connect to host
    let host = url.host_str().unwrap();
    let port = url.port_or_known_default().unwrap();
    let stream = unwrap_or_return_err!(
        connect(host, port)
    );

    // send HTTP GET request
    let path = url.path();
    let request = format!(concat!(
        "GET {} HTTP/1.1\r\n",
        "Host: {}\r\n",
        "Connection: close\r\n\r\n",
    ), path, host);
    unwrap_or_return_err!(
        send_request(&stream, &request)
    );

    // recieve response
    recieve_response(&stream, callback)
}

// Request HTTP GET with proxy and process response stream with the callback function.
// The callback is called chunk by chunk.
fn request_http_get_with_proxy(
    url: &Url,
    proxy_url: &Url,
    callback: fn(&[u8])
) -> std::io::Result<()> {
    // connect to proxy
    let proxy_host = proxy_url.host_str().unwrap();
    let proxy_port = proxy_url.port_or_known_default().unwrap();
    let stream = unwrap_or_return_err!(
        connect(proxy_host, proxy_port)
    );

    // send HTTP GET request
    let host = url.host_str().unwrap();
    let path = url.as_str();  // need to use the full URL as path when using proxy
    let request = format!(concat!(
        "GET {} HTTP/1.1\r\n",
        "Host: {}\r\n",
        "Connection: close\r\n\r\n",
    ), path, host);
    unwrap_or_return_err!(
        send_request(&stream, &request)
    );

    // recieve response
    recieve_response(&stream, callback)
}

// Connect to host server.
fn connect(host: &str, port: u16) -> std::io::Result<TcpStream> {
    TcpStream::connect((host, port)).map_err(|e| {
        std::io::Error::new(e.kind(), format!("Failed to connect host: {}", e))
    })
}

/// Send a request string to socket.
fn send_request(mut stream: &TcpStream, request: &str) -> std::io::Result<usize> {
    stream.write(request.as_bytes()).map_err(|e| {
        std::io::Error::new(e.kind(), format!("Failed to send request: {}", e))
    })
}

/// Recieve response chunk by chunk.
fn recieve_response(stream: &TcpStream, callback: fn(&[u8])) -> std::io::Result<()> {
    read_chunks(stream, callback).map_err(|e| {
        std::io::Error::new(e.kind(), format!("Failed to recieve response: {}", e))
    })
}

/// Maximum size of chunk.
const MAX_CHUNK_SIZE: usize = 1024;

/// Read bytes from a stream chunk by chunk and process it.
fn read_chunks(mut stream: &TcpStream, f: fn(&[u8])) -> std::io::Result<()> {
    let mut buf: [u8; MAX_CHUNK_SIZE] = [0; MAX_CHUNK_SIZE];
    loop {
        let read_size = unwrap_or_return_err!(stream.read(&mut buf));
        if read_size == 0 {
            return Ok(())
        } else {
            f(&buf[0..read_size]);
        }
    }
}
