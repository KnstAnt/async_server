use std::fs;
use std::io::prelude::*;
use std::time::Duration;
use std::io::Read;
use futures::stream::StreamExt;
use futures::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};
use std::fmt::format;

use serde::{Deserialize, Serialize};
use surrealdb::engine::remote::ws::Ws;
use surrealdb::opt::auth::Root;
use surrealdb::sql;
use surrealdb::sql::Thing;
use surrealdb::Surreal;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    create_db().await?;

    let listener = TcpListener::bind("127.0.0.1:7878").await?;

    loop {
        match listener.accept().await {
            Ok((socket, addr)) => {               
                println!("new client: {:?}", addr);

                if let Err(e) = process_socket(socket).await {
                    println!("couldn't process connection: {:?}", e);
                }
            },

            Err(e) => println!("couldn't get client: {:?}", e),
        }
    }
}


async fn process_socket<T>(mut socket: T) -> Result<(), Box<dyn std::error::Error>> 
    where T: tokio::io::AsyncReadExt + tokio::io::AsyncWriteExt + Unpin {
    let mut buffer = [0; 1024];
    socket.read(&mut buffer).await?;

    let get = b"GET / HTTP/1.1\r\n";
    let sleep = b"GET /sleep HTTP/1.1\r\n";

    let (status_line, filename) = if buffer.starts_with(get) {
        ("HTTP/1.1 200 OK\r\n\r\n", "hello.html")
    } else if buffer.starts_with(sleep) {
        tokio::time::sleep(Duration::from_millis(500)).await;
        ("HTTP/1.1 200 OK\r\n\r\n", "hello.html")
    } else {
        ("HTTP/1.1 404 NOT FOUND\r\n\r\n", "404.html")
    };

    let page = read_db().await?; //TODO

    let contents = fs::read_to_string(filename)?;

    let response = format!("{status_line}{contents}");
    socket.write(response.as_bytes()).await?;
    socket.flush().await?;

    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
struct Page{
    title: String,
    text: String,
}

#[derive(Debug, Deserialize)]
struct Record {
    #[allow(dead_code)]
    id: Thing,
}

async fn create_db() -> surrealdb::Result<()> {
    // Connect to the server
    let db = Surreal::new::<Ws>("127.0.0.1:8000").await?;

    // Signin as a namespace, database, or root user
    db.signin(Root {
        username: "root",
        password: "root",
    })
    .await?;

    // Select a specific namespace / database
    db.use_ns("test").use_db("test").await?;

    let created: Record = db
        .create("page")
        .content(Page {
            title: "Hello!".to_string(),
            text: "Hi from Rust".to_string(),
        })
    .await?;
    dbg!(created);

    Ok(())
}


async fn read_db() -> Result<Page, surrealdb::Error> {
    // Connect to the server
    let db = Surreal::new::<Ws>("127.0.0.1:8000").await?;

    // Signin as a namespace, database, or root user
    db.signin(Root {
        username: "root",
        password: "root",
    })
    .await?;

    // Select a specific namespace / database
    db.use_ns("test").use_db("test").await?;

    // Select all people records
    let page: Vec<Record> = db.select("page").await?;
    dbg!(page);

    let mut response = db
        .query("SELECT text FROM type::table($table)")
        .bind(("table", "page"))
        .await?;

    //    dbg!(response); 

    let page: Option<Page> = response.take(0)?;

    if let Some(page) = page {
        return Ok(page);
    }

    Ok(Page{ title: "empty".to_owned(), text: "page not found".to_owned() })
}