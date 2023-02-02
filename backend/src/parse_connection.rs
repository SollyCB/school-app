use tokio::net::TcpStream;
use tokio::io::AsyncReadExt;
use bytes::BytesMut;
use tokio::time::{Duration, sleep};
use crate::HttpRequest;


pub struct Connection {
    stream: Option<TcpStream>, 
    buf: Option<BytesMut>,
}

#[derive(Debug)]
pub enum RequestError {
    UnknownLength,
    FailedToReadRequest,
    CouldNotParseToString,
    EmptyRequest,
    NoMethod,
    UriAbsent,
    NoVersion,
    NoRequestLine,
    NoHeaders,
    NoTcpStream,
    ConnectionClosed,
    PostTooLarge,
    Timeout,
    NoRequest,
}

impl Connection {
    pub async fn new(stream: TcpStream) -> Self  {
        let buf = Some(BytesMut::with_capacity(4096));
        let stream = Some(stream);
        Connection { stream, buf }
    }

    async fn get_bytes(&mut self) -> Result<&mut Self, RequestError> {
        loop {
            if self.buf.as_ref().unwrap().capacity() > 100000 { return Err(RequestError::PostTooLarge)  }
            match self.stream.as_mut().unwrap().read_buf(&mut self.buf.as_mut().unwrap()).await {
                Ok(_) => {
                    tokio::select! {
                        _ = sleep(Duration::new(8, 0)) => { 
                            return Err(RequestError::Timeout) 
                        }
                        end = self.check_buf() => { 
                            if end { break } 
                            continue;
                        }
                    };
                },
                Err(_) => return Err(RequestError::ConnectionClosed)
            }
        }
        Ok(self)
    }

    pub async fn read_connection(&mut self) -> Result<&mut Self, RequestError>  {
        tokio::select!{
            // Content-Length header == buf.len()
            _ = sleep(Duration::new(5, 0)) => {
                return Err(RequestError::Timeout)
            }
            read = self.get_bytes() => {
                return read
            }
        }
    }

    async fn check_buf(&self) -> bool {
        if self.buf.as_ref().unwrap().len() > 3 && &self.buf.as_ref().unwrap()[self.buf.as_ref().unwrap().len()-4..] == b"\r\n\r\n" { 
            return true
        }
        false
    }

    pub async fn build_request(&mut self) -> Result<HttpRequest, RequestError> {

        let buf = self.buf.take().unwrap();
        let (request, body) = match std::str::from_utf8(&buf) {
            Ok(split) => {
                let mut split = split.split("\r\n\r\n").into_iter();
                let request = if let Some(body) = split.next() { body }
                    else { return Err(RequestError::NoRequest) };
                let body = if let Some(body) = split.next() { body }
                    else { "" };
                (request, body)
            },
            Err(_) => return Err(RequestError::NoRequest)
        };

        let mut request_iter = request.split("\r\n").into_iter();

        // Get request line (status line) and split into components
        let request_line: Vec<&str> = if let Some(request_line) = request_iter.next() { 
            request_line.split(' ').collect()
        } else { return Err(RequestError::EmptyRequest) };

        // Turn request line into iterator to save position of the parse
        let mut request_line_iter = request_line.into_iter();

        // Parse the request line
        let method = if let Some(method) = request_line_iter.next() { method.to_string() }
            else { return Err(RequestError::NoMethod) };
        let uri = if let Some(uri) = request_line_iter.next() { uri.to_string() }
            else { return Err(RequestError::UriAbsent) };
        let version = if let Some(version) = request_line_iter.next() { version.to_string() }
            else { return Err(RequestError::NoVersion) };

        // Map on the rest of the request to get the headers 
        let mut content_len = 0;
        let headers: Vec<String> = request_iter.map(|header| { 
            let mut split = header.split(' ').into_iter();
            if let Some(header) = split.next() {
                if header == "Content-Length:" {
                    match split.next() {
                        Some(num) => {
                            if let Ok(num) = num.parse() {
                                content_len = num;
                            }
                        }
                        None => ()
                    }
                }
            }
            header.to_string()
        }).collect();

        let body = if method == "GET" { "".to_string() }
            else {
                match self.if_post(body, content_len).await {
                    Ok(body) => body,
                    Err(err) => return Err(err),
                }
            };

        let stream = self.stream.take().unwrap();
        Ok(HttpRequest {method , uri, version, headers, body, stream })
    }

    pub async fn if_post(&mut self, body: &str, content_len: usize) -> Result<String, RequestError> {
    let body:String = 
        if body.len() > content_len {
            self.buf = Some(BytesMut::with_capacity(4096));
            match self.get_bytes().await {
                Ok(more) => {
                    body.to_string().push_str( 
                        if let Ok(string) = std::str::from_utf8(&more.buf.take().unwrap()) { string }
                            else { return Err(RequestError::CouldNotParseToString) }
                    );
                    body.to_string()
                },
                Err(_) => return Err(RequestError::ConnectionClosed) 
            }
       } else { body.to_string() };

        Ok(body)
    }


}

#[cfg(test)]
mod tests {
    use tokio::net::TcpListener;
    use super::Connection;

    #[ignore]
    #[tokio::test]
    async fn build_request() {
        let connection = TcpListener::bind("127.0.0.1:9000").await.expect("HERE");
        let (stream, _) = connection.accept().await.expect("CHILLI");
        let request = Connection::new(stream).await.read_connection().await.expect("Parsnips!!").build_request().await.expect("HUUUUHHH?");
        println!("{:?}", request);

        //panic!("I Panicked!");
        
        // The request:- HttpRequest { method: "GET", uri: "/api", version: "HTTP/1.1", headers: ["Host: localhost:9000"], body: Some(""), stream: Take { inner: PollEvented { io: Some(TcpStream { addr: 127.0.0.1:9000, peer: 127.0.0.1:49164, fd: 10 }) }, limit_: 0 } })
    }
}

