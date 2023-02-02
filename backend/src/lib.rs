pub mod parse_connection;
pub mod sql;

use serde::{Deserialize, Serialize};
use tokio::{
    net::TcpStream,
    io::AsyncWriteExt,
};
use std::str::Split;

#[derive(Debug)]
pub struct HttpRequest {
    method: String,
    uri: String,
    version: String,
    headers: Vec<String>,
    body: String,
    stream: TcpStream,
}

#[derive(Debug, Serialize)]
pub struct HttpResponse {
    status: String,
    version: String,
    headers: Vec<String>,
    body: String,
}

#[derive(Debug, Serialize)]
pub enum Body {
    Reports(sql::Reports),
    Class(sql::Class),
    Classes(sql::Classes),
    Pupils(sql::Pupils),
}

#[derive(Debug, Serialize)]
pub enum BadRequest {
    Cookie,             //  delete basically all of these and replace 
    Group,              //  with more concise struct type enums
    DB,
    NoTerm,
    InvalidGroup,
    Reports,
    GetReports,
    TermSubject,
    Params,
    Pupils,
    NotFound,
    Forbidden,
    Login,
    Body,
}

impl HttpRequest {
    
    pub async fn build(method: String, uri: String, version: String, headers: Vec<String>, body: String, stream: TcpStream ) -> HttpRequest {
        HttpRequest { method , uri, version, headers, body, stream }
    }

    fn cookie(&self) -> Option<()> {
        let cookie = for header in &self.headers {
            let mut header = header.split(' ');
            if header.next() == Some("Cookie:") { 
                return Some(())
            } 
        };
        println!("{:?}", cookie);
        None
        /*
           TODO:
            - check school, teacher permissions with cookie
        */
    }

    pub async fn class(&self, _school: &str, teacher: &str, mut params: Split<'_, char>) -> Result<Body, BadRequest> {
        let class = if let Some(id) = params.next() {
            sql::Class::new(id.to_string())
        } else { return Ok(Body::Classes(sql::Classes::new(teacher.to_string())))};

        let conn = if let Ok(conn) = sql::DB::new().await { conn.conn() } 
            else { return Err(BadRequest::DB) };

        if let Some(reports) = params.next() {
            match reports {
                "reports" => {
                    let subject = if let Some(subject) = params.next() { subject }
                        else { return Err(BadRequest::TermSubject) };
                    let terms: Vec<&str> = params.collect();
                    match class.reports(conn, subject, terms).await {
                        Ok(reports) => Ok(Body::Reports(reports)),
                        Err(_) => Err(BadRequest::Reports)
                    }
                },
                _ => return Err(BadRequest::Params)
            }
        } else { 
            let pupils = if let Ok(pupils) = class.pupils(conn).await { pupils }
                else { return Err(BadRequest::Pupils) };
            return Ok(Body::Pupils(pupils)) 
        }
    }

    pub async fn home(&self, school: &str, teacher: &str) -> Result<Body, BadRequest> {
        todo!()
    }

}

impl HttpResponse {

    pub async fn build(request: HttpRequest) -> Result<(HttpResponse, TcpStream), BadRequest> {
        match request.method.as_str() {
            "GET" => HttpResponse::get(request).await,
            "POST" => HttpResponse::post(request).await,
            _ => {
                println!("404 METHOD");
                Err(BadRequest::NotFound)
            },
        }
    }

    async fn get(request: HttpRequest) -> Result<(HttpResponse, TcpStream), BadRequest> {
        let mut uri = request.uri.split('/');
        uri.next();
        /*
            /<school_name>/<teacher_name>/<pupil>, <class>/<id>/info, reports/<subject>/ <- if reports
            /<school_name>/<teacher_name>/(params:-)<pupil, class>+<id>+<info, reports>+<subject>+<term> <- if reports
        */
        let (school, teacher) = if let (Some(school), Some(teacher)) = (uri.next(), uri.next()) { (school, teacher) } 
            else { 
                println!("404 school teacher");
                return Err(BadRequest::Login) 
            };

        //let params = uri.next().map(|params| params.split('+'));

        let body = if let Some(params) = uri.next() {
            let mut params = params.split('+');
            match params.next().expect("SPLITTING PARAMS") {
                "class" => request.class(school, teacher, params).await,
                "" =>  request.home(school, teacher).await,
                _ => {
                    println!("404 in BODY"); 
                    return Err(BadRequest::NotFound)
                },
            }
        } else { request.home(school, teacher).await };

        // Check for err in body and early return appropriately
        let body = if let Ok(body) = body { HttpResponse::body(body) }
            else { return Err(BadRequest::NotFound) };

        let length = body.as_bytes().len();

        let (status, version) = ("200 OK".to_string(), "HTTP/1.1".to_string());
        let headers = vec![format!("Content-Length: {}", length)];


        Ok( (HttpResponse { status, version, headers, body }, request.stream) )
    }

    async fn post(request: HttpRequest) -> Result<(HttpResponse, TcpStream), BadRequest> {
        println!("{}", request.body);
        todo!()
    }

    fn body<T: Serialize>(body: T) -> String {
        serde_json::to_string(&body).expect("SERDE SERIALIZE ON BODY")
    }

    pub async fn write(self, mut stream: TcpStream) -> tokio::io::Result<()> {
        let response = format!("{} {}\r\n{}\r\n\r\n{}\r\n\r\n", self.version, self.status, self.headers.join("\r\n"), self.body);
        stream.write_all(response.as_bytes()).await?;
        Ok(())
    }
}

