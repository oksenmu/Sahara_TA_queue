use futures_util::stream::SplitSink;
use warp::reply::Reply;
use warp::Filter;
use warp::ws::{Message as WarpMessage, WebSocket} ;
use warp::http::header::{CONTENT_TYPE, HeaderValue};
use futures_util::{StreamExt, SinkExt};
use tokio::sync::{mpsc, Mutex, MutexGuard};
use jsonwebtoken::{decode, encode, DecodingKey, Validation, Header, EncodingKey};
use serde::{Deserialize, Serialize};
use log::{debug, error, info, trace, warn};
use std::net::Ipv4Addr;
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::HashMap;
use std::sync::Arc;
use std::cmp::max;
use std::env;

// Shared list of connected clients

static IP: std::net::Ipv4Addr = Ipv4Addr::new(0, 0, 0, 0);
static PORT: u16 = 3030;

// legge til token her
struct QueueElement{
    getting_help: bool,
    queue_index: Option<i32>,
    table_number: u32,
    helped_by: String,
    task: String,
    id: u32,
    next: Option<u32>,
    previous: Option<u32>,
    socket: Option<mpsc::UnboundedSender<ThreadCommand>>
}

// Legge til token her?
#[derive(Debug, Deserialize, Serialize)]
struct TokenData {
    ta: bool,
    table_number: u32,
    name: String,
    task: String,
    helped_by: String,
    id: u32
}

#[derive(Debug, Deserialize, Serialize)]
struct Claims {
    user: TokenData,
    exp: usize
}

#[derive(Debug, Deserialize, Serialize)]
struct WebsoccketMessage {
    error: String,
    data: String,
    message_type: OutType
}

#[derive(Debug, Deserialize, Serialize)]
struct TableNumberUserMessage {
    table_number: u32,
    task: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct Name {
    error: String,
    data: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct TableNumber {
    error: String,
    data: u32,
}

#[derive(Debug, Deserialize, Serialize)]
struct StudentCommand<'a> {
    command: &'a str,
}

#[derive(Debug, Deserialize, Serialize)]
struct TaCommand<'a> {
    command: &'a str,
    argument: u32,
    value: &'a str
}

#[derive(Debug, Deserialize, Serialize)]
enum OutType{
    Index,
    Helping,
    NotQueue,
    Info,
    Queue,
    Error
}

enum ThreadCommand{
    Shut,
    Send(WarpMessage)
}

type TAs = Arc<Mutex<HashMap<u32, mpsc::UnboundedSender<ThreadCommand>>>>;
type Queue = Arc<Mutex<HashMap<u32, QueueElement>>>;
type QueueIndex = Arc<Mutex<Option<u32>>>;

// // Get JWT secret from environment or fallback to default
fn get_jwt_secret() -> String {
    env::var("JWT_PASSWORD").unwrap_or_else(|_| "supersecretkey".to_string())
}

fn get_ta_token() -> String {
    env::var("TA_TOKEN").unwrap_or_else(|_| "ta_token".to_string())
}

fn get_access_token(cookie_header: &str) -> Option<&str> {
    cookie_header
        .split(';')
        .map(str::trim)
        .find_map(|cookie| {
            let (name, value) = cookie.split_once('=')?;
            if name == "access_token_cookie" {
                Some(value)
            } else {
                None
            }
        })
}

fn set_jwt_cookie<R: Reply>(response: R, user: TokenData) -> impl Reply {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let claims = Claims {
        user,
        exp: (now + 60 * 60 * 24) as usize, // 1 day
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(get_jwt_secret().as_bytes()),
    ).unwrap();

    warp::reply::with_header(
        response,
        "Set-Cookie",
        format!(
            // "access_token_cookie={}; Max-Age=86400; Path=/; HttpOnly; SameSite=Strict",
            "access_token_cookie={}; Max-Age=86400; Path=/; SameSite=Strict",
            token
        ),
    )
}

fn validate_student_cookie(cookie: Option<String>) -> Option<TokenData> {
    let jwt_secret = get_jwt_secret();
    if let Some(cookie_str) = cookie {
        let token = get_access_token(&cookie_str)?;
        match decode::<Claims>(
            token,
            &DecodingKey::from_secret(jwt_secret.as_ref()),
            &Validation::default()
        ) {
            Ok(token_data) => {
                let token_data = token_data.claims.user;
                Option::Some(token_data)
            }
            Err(err) => {
                warn!("Cookie validation failed: {:?}", err);
                None
            },
        }
    } else {
        warn!("No Cookie");
        None
    }
}

fn validate_ta_cookie(cookie: Option<String>) -> Option<TokenData> {
    let jwt_secret = get_jwt_secret();
    if let Some(cookie_str) = cookie {
        let token = get_access_token(&cookie_str)?;
        match decode::<Claims>(
            token,
            &DecodingKey::from_secret(jwt_secret.as_ref()),
            &Validation::default()
        ) {
            Ok(token_data) => {
                let token_data = token_data.claims.user;
                if token_data.ta {
                    Option::Some(token_data)
                } else {
                    warn!("Is not TA");
                    None
                }
            }
            Err(err) => {
                warn!("TA cookie validation failed {:?}", err);
                None
            },
        }
    } else {
        warn!("No cookie");
        None
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let teaching_assistants: TAs = Arc::new(Mutex::new(HashMap::new()));
    let queue: Queue = Arc::new(Mutex::new(HashMap::new()));
    let q_head: QueueIndex = Arc::new(Mutex::new(Option::None));
    let q_tail: QueueIndex = Arc::new(Mutex::new(Option::None));

    let teaching_assistants_clone = teaching_assistants.clone();
    let queue_clone = queue.clone();
    let q_head_clone = q_head.clone();
    let q_tail_clone = q_tail.clone();

    let file_index_html = tokio::fs::read("frontend/index.html")
        .await
        .expect("failed to read file");
    let file_index_html = String::from_utf8(file_index_html).expect("Bytes should be valid utf8");
    // let file_index_html = bytes::Bytes::from(file_index_html);
    let file_index_js = tokio::fs::read("frontend/index.js")
        .await
        .expect("failed to read file");
    let file_index_js = bytes::Bytes::from(file_index_js);
    let file_styles_css = tokio::fs::read("frontend/styles.css")
        .await
        .expect("failed to read file");
    let file_styles_css = bytes::Bytes::from(file_styles_css);

    let file_student_js = tokio::fs::read("frontend/student.js")
        .await
        .expect("failed to read file");
    let file_student_js = bytes::Bytes::from(file_student_js);


    let file_ta_html = tokio::fs::read("frontend/ta.html")
        .await
        .expect("failed to read file");
    let file_ta_html = bytes::Bytes::from(file_ta_html);

    let file_admin_html = tokio::fs::read("frontend/admin.html")
        .await
        .expect("failed to read file");
    let file_admin_html = String::from_utf8(file_admin_html).expect("Bytes should be valid utf8");
    // let file_admin_html = bytes::Bytes::from(file_admin_html);
    let file_admin_js = tokio::fs::read("frontend/admin.js")
        .await
        .expect("failed to read file");
    let file_admin_js = bytes::Bytes::from(file_admin_js);

    let file_rooms_sahara_html = tokio::fs::read("frontend/rooms/sahara.html")
        .await
        .expect("failed to read file");
    let file_rooms_sahara_html = String::from_utf8(file_rooms_sahara_html).expect("Bytes should be valid utf8");

    let sahara_student_page = file_index_html.replace("%%ROOM%%", file_rooms_sahara_html.as_str());
    let sahara_student_page = bytes::Bytes::from(sahara_student_page);


    let sahara_admin_page = file_admin_html.replace("%%ROOM%%", file_rooms_sahara_html.as_str());
    let sahara_admin_page = bytes::Bytes::from(sahara_admin_page);

    let file_rooms_sahara_css = tokio::fs::read("frontend/rooms/sahara.css")
        .await
        .expect("failed to read file");
    let file_rooms_sahara_css = bytes::Bytes::from(file_rooms_sahara_css);

    info!("Starting webserver.....");


    // let index = serve_page(file_index_html, warp::path!(), "text/html");
    let index_js = serve_page(file_index_js, warp::path!("index.js"), "application/javascript");
    let student_js = serve_page(file_student_js, warp::path!("student.js"), "application/javascript");
    let styles = serve_page(file_styles_css, warp::path!("styles.css"), "text/css");
    let ta = serve_page(file_ta_html, warp::path!("ta"), "text,html");
    // let admin = serve_admin_page(file_admin_html, warp::path!("admin"), "text/html");
    let admin_js = serve_admin_page(file_admin_js, warp::path!("admin.js"), "application/javascript");
    let rooms_sahara = serve_page(sahara_student_page, warp::path!("rooms" / "sahara"), "text/html");
    let admin_rooms_sahara = serve_admin_page(sahara_admin_page, warp::path!("admin" / "rooms" / "sahara"), "text/html");
    let rooms_sahara_css = serve_page(file_rooms_sahara_css, warp::path!("rooms" / "sahara.css"), "text/css");

    let index = warp::path!()
        .and(warp::get())
        .map(|| {
            warp::redirect::temporary(
                warp::http::Uri::from_static("/rooms/sahara")
            )
        });
    let admin = warp::path!("admin")
        .and(warp::get())
        .map(|| {
            warp::redirect::temporary(
                warp::http::Uri::from_static("/admin/rooms/sahara")
            )
        });

    let favicon = warp::path!("favicons" / String)
        .and(warp::get())
        .and_then(|
            name: String,
        | async move {
            if name.contains('/') || name.contains('\\') || name.contains("..") {
                return Err(warp::reject::not_found());
            }
            let ficon = tokio::fs::read(format!("frontend/favicons/{}", name))
                .await
                .expect("failed to read file");
            let ficon = bytes::Bytes::from(ficon);
            let mut response = warp::reply::Response::new(ficon.into());
            if name == "0.gif"{
                response.headers_mut().insert(
                    CONTENT_TYPE,
                     HeaderValue::from_static("img/gif"),
                );
            } else{
                response.headers_mut().insert(
                    CONTENT_TYPE,
                     HeaderValue::from_static("img/jpg"),
                );
            }
            Ok::<_, warp::Rejection>(response)
        });

    let student_route = warp::path("student")
        .and(warp::ws())
        .and(warp::header::optional::<String>("cookie"))
        .and(warp::any().map(move || teaching_assistants_clone.clone()))
        .and(warp::any().map(move || queue_clone.clone()))
        .and(warp::any().map(move || q_head_clone.clone()))
        .and(warp::any().map(move || q_tail_clone.clone()))
        .map(|
            ws: warp::ws::Ws,
            cookie: Option<String>,
            teaching_assistants: TAs,
            queue: Queue, 
            q_head: QueueIndex,
            q_tail: QueueIndex,
        |{
            if let Some(cookie_value) = validate_student_cookie(cookie) {
                warp::reply::with_status(
                    ws.on_upgrade(move |socket| handle_student_websocket(
                        socket,
                        teaching_assistants,
                        queue,
                        q_head,
                        q_tail,
                        cookie_value
                    )),
                    warp::http::StatusCode::SWITCHING_PROTOCOLS
                ).into_response()
            } else {
                warn!("No some cookie value");
                warp::reply::with_status("", warp::http::StatusCode::FORBIDDEN).into_response()
            }
        });

    let ta_route = warp::path("ta_ws")
        .and(warp::ws())
        .and(warp::header::optional::<String>("cookie"))
        .and(warp::any().map(move || teaching_assistants.clone()))
        .and(warp::any().map(move || queue.clone()))
        .and(warp::any().map(move || q_head.clone()))
        .and(warp::any().map(move || q_tail.clone()))
        .map(|
            ws: warp::ws::Ws,
            cookie: Option<String>,
            teaching_assistants: TAs,
            queue: Queue, 
            q_head: QueueIndex,
            q_tail: QueueIndex,
        |{
            if let Some(cookie_value) = validate_ta_cookie(cookie) {
                warp::reply::with_status(
                    ws.on_upgrade(move |socket| handle_ta_websocket(
                        socket,
                        teaching_assistants,
                        queue,
                        q_head,
                        q_tail,
                        cookie_value
                    )),
                    warp::http::StatusCode::SWITCHING_PROTOCOLS
                ).into_response()
            } else {
                warp::reply::with_status("", warp::http::StatusCode::FORBIDDEN).into_response()
            }
        });

    let post_ta = warp::path!("ta")
        .and(warp::post())
        .and(warp::header::optional::<String>("cookie"))
        .and(warp::body::bytes())
        .map(|
            cookie: Option<String>,
            body: bytes::Bytes
        |{
            if let Some(mut cookie_value) = validate_student_cookie(cookie) {
                let token = match std::str::from_utf8(&body) {
                    Ok(token) => token.to_string(),
                    Err(_) => {
                        return warp::reply::with_status(
                            "Invalid request",
                            warp::http::StatusCode::BAD_REQUEST,
                        )
                        .into_response();
                    }
                };
                if token != get_ta_token(){
                    return warp::reply::with_status("", warp::http::StatusCode::FORBIDDEN).into_response();
                }
                cookie_value.ta = true;
                let response = warp::reply::json(&Name {
                    error: String::new(),
                    data: cookie_value.name.clone(),
                });
                set_jwt_cookie(response, cookie_value).into_response()
            } else {
                warp::reply::with_status("", warp::http::StatusCode::FORBIDDEN).into_response()
            }
        });


    let api_task_route = warp::path!("api" / "task")
        .and(warp::get())
        .and(warp::header::optional::<String>("cookie"))
        .map(|
            cookie: Option<String>,
        |{
            if let Some(cookie_value) = validate_student_cookie(cookie) {
                let out = Name{
                    error: "".to_string(),
                    data: cookie_value.task,
                };
                warp::reply::json(
                    &out
                ).into_response()
            } else {
                warp::reply::with_status("", warp::http::StatusCode::FORBIDDEN).into_response()
            }
        });

    let post_api_task_route = warp::path!("api" / "task")
        .and(warp::post())
        .and(warp::header::optional::<String>("cookie"))
        .and(warp::body::bytes())
        .map(|
            cookie: Option<String>,
            body: bytes::Bytes
        |{
            if let Some(mut cookie_value) = validate_student_cookie(cookie) {
                let task = match std::str::from_utf8(&body) {
                    Ok(task) => task.to_string(),
                    Err(_) => {
                        return warp::reply::with_status(
                            "Invalid request",
                            warp::http::StatusCode::BAD_REQUEST,
                        )
                        .into_response();
                    }
                };
                cookie_value.task = task;
                let response = warp::reply::json(&Name {
                    error: String::new(),
                    data: cookie_value.task.clone(),
                });
                set_jwt_cookie(response, cookie_value).into_response()
            } else {
                warp::reply::with_status("", warp::http::StatusCode::FORBIDDEN).into_response()
            }
        });

    let api_name_route = warp::path!("api" / "name")
        .and(warp::get())
        .and(warp::header::optional::<String>("cookie"))
        .map(|
            cookie: Option<String>,
        |{
            if let Some(cookie_value) = validate_ta_cookie(cookie) {
                let out = Name{
                    error: "".to_string(),
                    data: cookie_value.name,
                };
                warp::reply::json(
                    &out
                ).into_response()
            } else {
                warp::reply::with_status("", warp::http::StatusCode::FORBIDDEN).into_response()
            }
        });

    let post_api_name_route = warp::path!("api" / "name")
        .and(warp::post())
        .and(warp::header::optional::<String>("cookie"))
        .and(warp::body::bytes())
        .map(|
            cookie: Option<String>,
            body: bytes::Bytes
        |{
            if let Some(mut cookie_value) = validate_ta_cookie(cookie) {
                let name = match std::str::from_utf8(&body) {
                    Ok(name) => name.to_string(),
                    Err(_) => {
                        return warp::reply::with_status(
                            "Invalid request",
                            warp::http::StatusCode::BAD_REQUEST,
                        )
                        .into_response();
                    }
                };
                cookie_value.name = name;
                let response = warp::reply::json(&Name {
                    error: String::new(),
                    data: cookie_value.name.clone(),
                });
                set_jwt_cookie(response, cookie_value).into_response()
            } else {
                warp::reply::with_status("", warp::http::StatusCode::FORBIDDEN).into_response()
            }
        });

    let api_table_number_route = warp::path!("api" / "table_number")
        .and(warp::get())
        .and(warp::header::optional::<String>("cookie"))
        .map(|
            cookie: Option<String>,
        |{
            if let Some(cookie_value) = validate_student_cookie(cookie) {
                let out = TableNumber{
                    error: "".to_string(),
                    data: cookie_value.table_number,
                };
                warp::reply::json(
                    &out
                ).into_response()
            } else {
                warn!("No some cookie value");
                warp::reply::with_status("", warp::http::StatusCode::FORBIDDEN).into_response()
            }
        });

    let post_api_table_number_route = warp::path!("api" / "table_number")
        .and(warp::post())
        .and(warp::header::optional::<String>("cookie"))
        .and(warp::body::json::<TableNumberUserMessage>())
        .map(|
            cookie: Option<String>,
            message: TableNumberUserMessage
        |{
            if let Some(mut cookie_value) = validate_student_cookie(cookie) {
                cookie_value.table_number = message.table_number;
                cookie_value.task = message.task;
                let response = warp::reply::json(&TableNumber {
                    error: String::new(),
                    data: cookie_value.table_number,
                });
                set_jwt_cookie(response, cookie_value).into_response()
            } else {
                warn!("No some cookie value");
                warp::reply::with_status("", warp::http::StatusCode::FORBIDDEN).into_response()
            }
        });


    let routes = index
        .or(index_js)
        .or(student_js)
        .or(styles)
        .or(admin)
        .or(admin_js)
        .or(rooms_sahara)
        .or(admin_rooms_sahara)
        .or(rooms_sahara_css)
        .or(ta)
        .or(post_ta)
        .or(student_route)
        .or(ta_route)
        .or(api_name_route)
        .or(post_api_name_route)
        .or(api_task_route)
        .or(post_api_task_route)
        .or(api_table_number_route)
        .or(post_api_table_number_route)
        .or(favicon)
        .with(warp::log("http"));

    warp::serve(routes)
        .run((IP, PORT))
        .await
}

fn serve_page<F>(
    file: bytes::Bytes,
    path: F,
    content_type: &'static str,
) -> impl Filter<
    Extract = (warp::reply::Response,),
    Error = warp::Rejection,
> + Clone
where
    F: Filter<Extract = (), Error = warp::Rejection> + Clone,
{
    path
        .and(warp::get())
        .and(warp::header::optional::<String>("cookie"))
        .map(move |
            cookie: Option<String>,
        |{
            let mut response = warp::reply::Response::new(file.clone().into());
            response.headers_mut().insert(
                CONTENT_TYPE,
                 HeaderValue::from_static(content_type),
            );
            if let Some(cookie_value) = validate_student_cookie(cookie) {
                set_jwt_cookie(response, cookie_value).into_response()
            } else {
                let cookie_value = default_user();
                set_jwt_cookie(response, cookie_value).into_response()
            }
        })
}

fn serve_admin_page<F>(
    file: bytes::Bytes,
    path: F,
    content_type: &'static str,
) -> impl Filter<
    Extract = (warp::reply::Response,),
    Error = warp::Rejection,
> + Clone
where
    F: Filter<Extract = (), Error = warp::Rejection> + Clone,
{
    path
        .and(warp::get())
        .and(warp::header::optional::<String>("cookie"))
        .map(move |
            cookie: Option<String>,
        |{
            let mut response = warp::reply::Response::new(file.clone().into());
            response.headers_mut().insert(
                CONTENT_TYPE,
                 HeaderValue::from_static(content_type),
            );
            if let Some(cookie_value) = validate_ta_cookie(cookie) {
                set_jwt_cookie(response, cookie_value).into_response()
            } else {
                warp::reply::with_status("", warp::http::StatusCode::FORBIDDEN).into_response()
            }
        })
}


async fn handle_student_websocket(
    ws: warp::ws::WebSocket,
    teaching_assistants: TAs,
    queue: Queue,
    q_head: QueueIndex,
    q_tail: QueueIndex,
    cookie: TokenData
){
    debug!("Handeling Student WebSocket");
    let (mut tx, mut rx) = ws.split();
    let (msg_tx, mut msg_rx) = mpsc::unbounded_channel();
    let table_number = cookie.table_number;
    let task = cookie.task;
    let id = cookie.id;
    // if new socket already exists then shut that down and create new
    let mut queue_lock = lock("queue", &queue).await;

    if let Some(queue_element) = queue_lock.get_mut(&cookie.id){
        if let Some(old_msg_tx) = queue_element.socket.take(){
            let result = old_msg_tx.send(ThreadCommand::Shut);
            if let Err(err) = result{
                error!("Failed to send websocket message: {}", err)
            }
        }
        queue_element.task = task.clone();
        queue_element.table_number = table_number;
        queue_element.socket = Some(msg_tx.clone());
    }else {
        let queue_element = QueueElement{
            getting_help: false,
            queue_index: None,
            helped_by: "".to_string(),
            task: task.clone(),
            id,
            table_number,
            next: None,
            previous: None,
            socket: Some(msg_tx.clone()),
        };
        queue_lock.insert(id, queue_element);
    }
    release("queue_lock", queue_lock);
    loop {
        tokio::select! {
            Some(result) = rx.next() => {
                if let Ok(msg) = result {
                    if msg.is_text() {
                        let msg_clone = msg.clone();
                        let student_command: Result<StudentCommand, serde_json::Error> = serde_json::from_str(msg_clone.to_str().unwrap());
                        if let Ok(command) = student_command{
                            match command.command {
                                "poll" => poll_queue(&mut tx, &msg_tx, &teaching_assistants, &queue, &q_head, id).await,
                                "join" => join_queue(&mut tx, &teaching_assistants, &queue, &q_head, &q_tail, id).await,
                                "leave" => leave_queue(&mut tx, &teaching_assistants, &queue, &q_head, &q_tail, id).await,
                                _ => tx.send(WarpMessage::text(serde_json::to_string(
                                    &WebsoccketMessage{
                                        error: "Invalid command".to_string(),
                                        message_type: OutType::Error,
                                        data: "".to_string()
                                    }
                                ).unwrap())).await.unwrap(),
                            };
                        } else{
                            let result = tx.send(WarpMessage::text("Not text")).await;
                            if let Err(err) = result{
                                error!("Failed to send websocket message: {}", err)
                            }
                        }
                    }
                }
            }
            Some(info) = msg_rx.recv() => {
                if let ThreadCommand::Shut = info{
                    debug!("Closeing connection");
                    let mut queue_lock = lock("queue", &queue).await;

                    let queue_element = queue_lock.get_mut(&cookie.id);
                    if queue_element.is_none(){
                        release("queue_lock", queue_lock);
                        return;
                    }
                    let queue_element = queue_element.unwrap();
                    if queue_element.queue_index.is_none(){
                        queue_lock.remove(&id);
                    }
                    release("queue_lock", queue_lock);
                    return;
                }
                else if let ThreadCommand::Send(data) = info{
                    let _ = tx.send(data).await;
                }
            }
        }
    }
}

async fn leave_queue(
    tx: &mut SplitSink<WebSocket, WarpMessage>,
    teaching_assistants: &TAs,
    queue: &Queue,
    q_head: &QueueIndex,
    q_tail: &QueueIndex,
    id: u32
) {
    debug!("Leave queue deteced");
    let queue_lock = lock("queue", &queue).await;

    if let Some(_) = queue_lock.get(&id){
        release("queue_lock", queue_lock);
        remove_student(tx, teaching_assistants, queue, q_head, q_tail, id).await;
    } else  {
        let out = WebsoccketMessage{
            error: "Not in queue".to_string(),
            message_type: OutType::Error,
            data: "".to_string()
        };
        let result = tx.send(WarpMessage::text(serde_json::to_string(&out).unwrap())).await;
        if let Err(err) = result{
            error!("Failed to send websocket message: {}", err)
        }
        release("queue_lock", queue_lock);
    }
}

async fn join_queue(
    tx: &mut SplitSink<WebSocket, WarpMessage>,
    teaching_assistants: &TAs,
    queue: &Queue,
    q_head: &QueueIndex,
    q_tail: &QueueIndex,
    id: u32
    // take in token
){
    let value: i32;
    debug!("Join queue deteced");
    let mut queue_lock = lock("queue", &queue).await;
    let mut ta_lock = lock("teaching_assistants", &teaching_assistants).await;
    let mut q_head_lock = lock("q_head", &q_head).await;
    let mut q_tail_lock = lock("q_tail", &q_tail).await;


    let queue_element = queue_lock.get(&id).unwrap();

    if queue_element.queue_index.is_some(){
        let out = WebsoccketMessage{
            error: "Allredy in queue".to_string(),
            message_type: OutType::Error,
            data: "".to_string()
        };
        let result = tx.send(WarpMessage::text(serde_json::to_string(&out).unwrap())).await;
        if let Err(err) = result{
            error!("Failed to send websocket message: {}", err)
        }
        let queue = build_queue(queue_lock, q_head_lock);
        ta_lock.retain(|_, ta| {
            let out = WebsoccketMessage{
                error: "".to_string(),
                message_type: OutType::Queue,
                data: (&queue).to_string()
            };
            let message = WarpMessage::text(serde_json::to_string(&out).unwrap());
            ta.send(ThreadCommand::Send(message)).is_ok()
        });
        release("ta_lock", ta_lock);
        release("q_tail_lock", q_tail_lock);
        return;
    }
    if let Some(qt) = *q_tail_lock  {
        let tail = queue_lock.get_mut(&qt).unwrap();
        tail.next = Some(id);
        value = max(tail.queue_index.unwrap() + 1, 1);
        let queue_element = queue_lock.get_mut(&id).unwrap();
        queue_element.queue_index = Some(value);
        queue_element.previous = Some(qt);
    }
    else{
        value = 1;
        let queue_element = queue_lock.get_mut(&id).unwrap();
        queue_element.queue_index = Some(value);
        *q_head_lock = Some(id);
    }
    *q_tail_lock = Some(id);
    let out = WebsoccketMessage{
        error: "".to_string(),
        message_type: OutType::Index,
        data: value.to_string()
    };

    let result = tx.send(WarpMessage::text(serde_json::to_string(&out).unwrap())).await;
    if let Err(err) = result{
        error!("Failed to send websocket message: {}", err)
    }


    let queue = build_queue(queue_lock, q_head_lock);
    ta_lock.retain(|_, ta| {
        let out = WebsoccketMessage{
            error: "".to_string(),
            message_type: OutType::Queue,
            data: (&queue).to_string()
        };
        let message = WarpMessage::text(serde_json::to_string(&out).unwrap());
        ta.send(ThreadCommand::Send(message)).is_ok()
    });
    release("ta_lock", ta_lock);
    release("q_tail_lock", q_tail_lock);
}

async fn poll_queue(
    tx: &mut SplitSink<WebSocket, WarpMessage>,
    msg_tx: &mpsc::UnboundedSender<ThreadCommand>,
    teaching_assistants: &TAs,
    queue: &Queue,
    q_head: &QueueIndex,
    id: u32
){
    //Should send the new queue to TAs if allredy in queue to update table
    debug!("Polling detected");
    let queue_lock = lock("queue", &queue).await;
    let mut ta_lock = lock("teaching_assistants", &teaching_assistants).await;
    let q_head_lock = lock("q_head", &q_head).await;

    let queue_element = queue_lock.get(&id).unwrap();
    if queue_element.queue_index == None{
        let out = WebsoccketMessage{
            error: "".to_string(),
            message_type: OutType::NotQueue,
            data: "Not in queue".to_string()
        };
        let result = tx.send(WarpMessage::text(serde_json::to_string(&out).unwrap())).await;
        if let Err(err) = result{
            error!("Failed to send websocket message: {}", err)
        }
        let result = msg_tx.send(ThreadCommand::Shut);
        if let Err(err) = result{
            error!("Failed to send websocket message: {}", err)
        }
        release("queue_lock", queue_lock);
        return;
    }
    let out: WebsoccketMessage;
    if queue_element.getting_help{
        let data = format!("Getting help from {}", queue_element.helped_by);
        out = WebsoccketMessage{
            error: "".to_string(),
            message_type: OutType::Helping,
            data
        };
    } else {
        let data = format!("{}", queue_element.queue_index.unwrap());
        out = WebsoccketMessage{
            error: "".to_string(),
            message_type: OutType::Index,
            data
        };
    }
    let result = tx.send(WarpMessage::text(serde_json::to_string(&out).unwrap())).await;
    if let Err(err) = result{
        error!("Failed to send websocket message: {}", err)
    }

    let queue = build_queue(queue_lock, q_head_lock);
    ta_lock.retain(|_, ta| {
        let out = WebsoccketMessage{
            error: "".to_string(),
            message_type: OutType::Queue,
            data: (&queue).to_string()
        };
        let message = WarpMessage::text(serde_json::to_string(&out).unwrap());
        ta.send(ThreadCommand::Send(message)).is_ok()
    });
    release("ta_lock", ta_lock);
}

async fn handle_ta_websocket(
    ws: warp::ws::WebSocket,
    teaching_assistants: TAs,
    queue: Queue,
    q_head: QueueIndex,
    q_tail: QueueIndex,
    cookie: TokenData
){
    debug!("Handeling TA websockets");
    let (mut tx, mut rx) = ws.split();
    let (msg_tx, mut msg_rx) = mpsc::unbounded_channel();
    let mut name = cookie.name;
    let id = cookie.id;
    {
        let mut ta_lock = lock("teaching_assistants", &teaching_assistants).await;
        ta_lock.insert(id, msg_tx.clone());
        release("ta_lock", ta_lock);
    }
    loop {
        tokio::select! {
            Some(result) = rx.next() => {
                if let Ok(msg) = result {
                    if msg.is_text() {
                        let msg_clone = msg.clone();
                        let ta_command: Result<TaCommand, serde_json::Error> = serde_json::from_str(msg_clone.to_str().unwrap());
                        if let Ok(command) = ta_command{
                            match command.command {
                                "help" => help_student(&mut tx,  &teaching_assistants, &queue, &q_head, &q_tail, &name, command.argument).await,
                                "remove" => remove_student(&mut tx, &teaching_assistants, &queue, &q_head, &q_tail, command.argument).await,
                                "get_queue" => get_queue(&mut tx, &queue, &q_head).await,
                                "set_name" => set_name(&mut tx, command, &mut name).await,
                                _ => tx.send(WarpMessage::text(serde_json::to_string(
                                    &WebsoccketMessage{
                                        error: "Invalid command".to_string(),
                                        message_type: OutType::Error,
                                        data: "".to_string()
                                    }
                                ).unwrap())).await.unwrap()
                            };
                        }
                        else{
                            let result = tx.send(WarpMessage::text("Not text")).await;
                            if let Err(err) = result{
                                error!("Failed to send websocket message: {}", err)
                            }
                        }
                    }
                }
            }
            Some(info) = msg_rx.recv() => {
                if let ThreadCommand::Shut = info{
                    let mut ta_lock = lock("teaching_assistants", &teaching_assistants).await;
                    ta_lock.remove(&id);
                    release("ta_lock", ta_lock);
                    return;
                }
                else if let ThreadCommand::Send(data) = info{
                    let _ = tx.send(data).await;
                }
            }
        }
    }
}

async fn help_student(
    tx: &mut SplitSink<WebSocket, WarpMessage>,
    teaching_assistants: &TAs,
    queue: &Queue,
    q_head: &QueueIndex,
    q_tail: &QueueIndex,
    name: &String,
    id: u32,
){
    debug!("Help students deteced");
    let mut queue_lock = lock("queue", &queue).await;
    let mut ta_lock = lock("teaching_assistants", &teaching_assistants).await;
    let mut q_head_lock = lock("q_head", &q_head).await;
    let mut q_tail_lock = lock("q_tail", &q_tail).await;

    let queue_element = queue_lock.get_mut(&id).expect("No queue element for given id");

    //Not in queue
    if queue_element.queue_index.is_none() {
        let out = WebsoccketMessage {
            error: "Student not in queue".to_string(),
            message_type: OutType::Error,
            data: "".to_string(),
        };
        let result = tx.send(WarpMessage::text(serde_json::to_string(&out).unwrap())).await;
        if let Err(err) = result{
            error!("Failed to send websocket message: {}", err)
        }
        release("q_tail_lock", q_tail_lock);
        release("ta_lock", ta_lock);
        release("queue_lock", queue_lock);
        release("q_head_lock", q_head_lock);
        return;
    }

    //Allready getting help
    if queue_element.getting_help{
        let out = WebsoccketMessage {
            error: "Student already getting help".to_string(),
            message_type: OutType::Error,
            data: "".to_string(),
        };
        let result = tx.send(WarpMessage::text(serde_json::to_string(&out).unwrap())).await;
        if let Err(err) = result{
            error!("Failed to send websocket message: {}", err)
        }
        release("q_tail_lock", q_tail_lock);
        release("ta_lock", ta_lock);
        release("queue_lock", queue_lock);
        release("q_head_lock", q_head_lock);
        return;
    }

    let previous = queue_element.previous;
    let mut next = queue_element.next;
    let table_number = queue_element.table_number;


    //Sets it self as being helped
    queue_element.queue_index = Some(-1);
    queue_element.getting_help = true;
    queue_element.helped_by = name.clone();
    if let Some(ref helped_tx) = queue_element.socket{
        let data = format!("Getting help from {}", name);
        let out = WebsoccketMessage{
            error: "".to_string(),
            message_type: OutType::Helping,
            data
        };
        let data = WarpMessage::text(serde_json::to_string(&out).unwrap());
        let result = helped_tx.send(ThreadCommand::Send(data));
        if let Err(err) = result{
            error!("Failed to send websocket message: {}", err)
        }
    }

    // Reconnect the queue
    if let Some(next_element) = next{
        let next_element = queue_lock.get_mut(&next_element).unwrap();
        next_element.previous = previous;
    }
    if let Some(prev_element) = previous{
        let prev_element = queue_lock.get_mut(&prev_element).unwrap();
        prev_element.next = next;
    }

    let queue_element = queue_lock.get_mut(&id).unwrap();

    // Adds it self to the front of the queue
    if let Some(q_head) = *q_head_lock && q_head != id{
        queue_element.next = Some(q_head);
        let old_head_element = queue_lock.get_mut(&q_head).unwrap();
        old_head_element.previous = Some(id);
    }
    let queue_element = queue_lock.get_mut(&id).unwrap();
    *q_head_lock = Some(id);
    queue_element.previous = None;

    //Updates the tail if neccesarry
    if let Some(q_tail) = *q_tail_lock && q_tail == id && previous.is_some(){
        *q_tail_lock = previous
    }

    //Inform about going up in the queue
    while let Some(q_next) = next{
        let q_next = queue_lock.get_mut(&q_next).unwrap();
        if q_next.queue_index.unwrap() <= 1 {
            break;
        } 
        q_next.queue_index = Some(q_next.queue_index.unwrap() - 1);
        if let Some(ref tx) = q_next.socket{
            let data = format!("{}", q_next.queue_index.unwrap());
            let out = WebsoccketMessage{
                error: "".to_string(),
                message_type: OutType::Index,
                data
            };
            let message = WarpMessage::text(serde_json::to_string(&out).unwrap());
            let result = tx.send(ThreadCommand::Send(message));
            if let Err(err) = result{
                error!("Failed to send websocket message: {}", err)
            }

        }
        next = q_next.next;
    }

    // Inform the student that it's getting help
    let data = format!("Now helping {}", table_number);
    let out = WebsoccketMessage {
        error: "".to_string(),
        message_type: OutType::Info,
        data
    };

    let result = tx.send(WarpMessage::text(serde_json::to_string(&out).unwrap())).await;
    if let Err(err) = result{
        error!("Failed to send websocket message: {}", err)
    }

    //Send the uppdated queue to the TAs
    let queue = build_queue(queue_lock, q_head_lock);
    ta_lock.retain(|_, ta| {
        let out = WebsoccketMessage{
            error: "".to_string(),
            message_type: OutType::Queue,
            data: (&queue).to_string()
        };
        let message = WarpMessage::text(serde_json::to_string(&out).unwrap());
        ta.send(ThreadCommand::Send(message)).is_ok()
    });
    release("q_tail_lock", q_tail_lock);
    release("ta_lock", ta_lock);
}

async fn remove_student(
    tx: &mut SplitSink<WebSocket, WarpMessage>,
    teaching_assistants: &TAs,
    queue: &Queue,
    q_head: &QueueIndex,
    q_tail: &QueueIndex,
    id: u32
){
    debug!("Remove student deteced");
    let mut queue_lock = lock("queue", &queue).await;
    let mut ta_lock = lock("teaching_assistants", &teaching_assistants).await;
    let mut q_head_lock = lock("q_head", &q_head).await;
    let mut q_tail_lock = lock("q_tail", &q_tail).await;

    let queue_element = queue_lock.get_mut(&id).unwrap();

    //Not in queue
    if queue_element.queue_index.is_none() {
        let out = WebsoccketMessage {
            error: "Student not in queue".to_string(),
            message_type: OutType::Error,
            data: "".to_string(),
        };
        let result = tx.send(WarpMessage::text(serde_json::to_string(&out).unwrap())).await;
        if let Err(err) = result{
            error!("Failed to send websocket message: {}", err)
        }
        print!("Droped lock");
        return;
    }

    let previous = queue_element.previous;
    let mut next = queue_element.next;
    let table_number = queue_element.table_number;

    // Updates the head if neccesarry
    if let Some(q_head) = *q_head_lock && q_head == id {
        *q_head_lock = queue_element.next;
    }
    queue_element.previous = None;

    //Updates the tail if neccesarry
    if let Some(q_tail) = *q_tail_lock && q_tail == id{
        *q_tail_lock = previous
    }
    //Informing TA
    let data = format!("Removing student {}", table_number);
    let out = WebsoccketMessage {
        error: "".to_string(),
        message_type: OutType::Info,
        data,
    };
    let result = tx.send(WarpMessage::text(serde_json::to_string(&out).unwrap())).await;
    if let Err(err) = result{
        error!("Failed to send websocket message: {}", err)
    }
    if let Some(ref tx)  = queue_element.socket{
        let out = WebsoccketMessage{
            error: "".to_string(),
            message_type: OutType::NotQueue,
            data: "Removed from queue".to_string()
        };
        let data = WarpMessage::text(serde_json::to_string(&out).unwrap());
        let result = tx.send(ThreadCommand::Send(data));
        if let Err(err) = result{
            error!("Failed to send websocket message: {}", err)
        }
        let result = tx.send(ThreadCommand::Shut);
        if let Err(err) = result{
            error!("Failed to send websocket message: {}", err)
        }
    }

    queue_lock.remove(&id);

    // Reconnect the queue
    if let Some(next_element) = next{
        let next_element = queue_lock.get_mut(&next_element).unwrap();
        next_element.previous = previous;
    }

    if let Some(prev_element) = previous{
        let prev_element = queue_lock.get_mut(&prev_element).unwrap();
        prev_element.next = next;
    }

    //Inform about going up in the queue
    while let Some(q_next) = next{
        let q_next = queue_lock.get_mut(&q_next).unwrap();
        if q_next.queue_index.unwrap() <= 1 {
            break;
        } 
        q_next.queue_index = Some(q_next.queue_index.unwrap() - 1);
        if let Some(ref tx) = q_next.socket{
            let data = format!("{}", q_next.queue_index.unwrap());
            let out = WebsoccketMessage{
                error: "".to_string(),
                message_type: OutType::Index,
                data
            };
            let message = WarpMessage::text(serde_json::to_string(&out).unwrap());
            let result = tx.send(ThreadCommand::Send(message));
            if let Err(err) = result{
                error!("Failed to send websocket message: {}", err)
            }

        }
        next = q_next.next;
    }

    let queue = build_queue(queue_lock, q_head_lock);
    ta_lock.retain(|_, ta| {
        let out = WebsoccketMessage{
            error: "".to_string(),
            message_type: OutType::Queue,
            data: (&queue).to_string()
        };
        let message = WarpMessage::text(serde_json::to_string(&out).unwrap());
        ta.send(ThreadCommand::Send(message)).is_ok()
    });
    release("q_tail_lock", q_tail_lock);
    release("ta_lock", ta_lock);
}


async fn get_queue(
    tx: &mut SplitSink<WebSocket, WarpMessage>,
    queue: &Queue,
    q_head: &QueueIndex,
){
    debug!("Get queue deteced");
    let queue_lock = lock("queue", &queue).await;
    let q_head_lock = lock("q_head", &q_head).await;

    let data = build_queue(queue_lock, q_head_lock);
    let out = WebsoccketMessage {
        error: "".to_string(),
        message_type: OutType::Queue,
        data
    };

    let result = tx.send(WarpMessage::text(serde_json::to_string(&out).unwrap())).await;
    if let Err(err) = result{
        error!("Failed to send websocket message: {}", err)
    }
}

async fn set_name<'a>(
    tx: &mut SplitSink<WebSocket, WarpMessage>,
    command: TaCommand<'a>,
    name: &mut String,
){
    *name = command.value.to_string();
    let out = WebsoccketMessage {
        error: "".to_string(),
        message_type: OutType::Info,
        data: name.clone()
    };

    let result = tx.send(WarpMessage::text(serde_json::to_string(&out).unwrap())).await;
    if let Err(err) = result{
        error!("Failed to send websocket message: {}", err)
    }
}

async fn lock<'a, T>(name: &str, mutex: &'a Arc<Mutex<T>>) -> MutexGuard<'a, T>{
    trace!("Aquireing {name} {:p} lock", mutex);
    let out = mutex.lock().await;
    trace!("Aquired {name} lock {:p}", mutex);
    out
}

fn release<'a, T>(name: &str, _mutex_guard: MutexGuard<'a, T>){
    trace!("Released {name}");
}

fn default_user() -> TokenData{
    TokenData{
        ta: false,
        helped_by: "".to_string(),
        name: "Teaching assistant".to_string(),
        table_number: 0,
        task: "".to_string(),
        id: rand::random::<u32>(),
    }
}

fn build_queue(
    queue_lock: MutexGuard<'_, HashMap<u32, QueueElement>>,
    q_head_lock: MutexGuard<'_, Option<u32>>,
) -> String{
    let mut queue = "".to_string();
    let mut itr_next_idx = *q_head_lock;
    while let Some(q_next_id) = itr_next_idx{
        let q_next = queue_lock.get(&q_next_id).expect("Invalid linked list");
        let item = serde_json::json!({
            "index": q_next.queue_index.expect("Index is none"),
            "table_number": q_next.table_number,
            "id": q_next.id,
            "task": q_next.task,
        });
        queue = format!("{queue}, {item}");
        itr_next_idx = q_next.next;
    }
    release("queue_lock", queue_lock);
    release("q_head_lock", q_head_lock);
    if queue.len() > 2{
        format!("[{}]", &queue[2..])
    }else {
        "[]".to_string()
    }
}
