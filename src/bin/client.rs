
use std::io::{Write, Read};  // need to use stream.read() and stream.write()
use std::net::TcpStream;
use url::Url;

/// Exit program with printing a message to stderr.
macro_rules! exit_with_message {
    ($($arg:tt)*) => {
        eprintln!($($arg)*);
        std::process::exit(-1);
    }
}

fn main() {
    // check command line arguments
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        exit_with_message!("Usage: {} <URL>", args[0]);
    }

    // parse and validate input URL
    let url = Url::parse(&args[1]).unwrap_or_else(|e| {
        exit_with_message!("Malformed URL: {}", e);
    });
    if url.scheme() != "http" {
        exit_with_message!("Unsupported shceme: {}", url.scheme());
    }

    let host = url.host_str().unwrap();
    let path = url.path();
    let port = url.port_or_known_default().unwrap();

    // connect to host
    let mut stream = TcpStream::connect((host, port)).unwrap_or_else(|e| {
        exit_with_message!("{}", e);
    });

    // send HTTP GET request
    let request = format!(concat!(
        "GET {} HTTP/1.1\r\n",
        "Host: {}\r\n",
        "Connection: close\r\n\r\n",
    ), path, host);
    stream.write(request.as_bytes()).unwrap_or_else(|e| {
        exit_with_message!("Failed to send request: {}", e);
    });

    // recieve response
    read_chunks(&stream, |chunk| {
        print!("{}", std::str::from_utf8(chunk).unwrap());
    }).unwrap_or_else(|e| {
        exit_with_message!("Failed to recieve response: {}", e);
    })
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
