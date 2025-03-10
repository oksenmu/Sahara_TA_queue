use futures_util::stream::SplitSink;
use warp::reply::Reply;
use warp::Filter;
use warp::ws::{Message as WarpMessage, WebSocket} ;
use futures_util::{StreamExt, SinkExt};
use tokio::sync::{mpsc, Mutex, MutexGuard};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;
use std::sync::Arc; use std::env;

// Shared list of connected clients

static IP: std::net::Ipv4Addr = Ipv4Addr::new(0, 0, 0, 0);
static PORT: u16 = 3030;

struct QueueElement{
    value: u32,
    helped_by: String,
    task: String,
    next: Option<u32>,
    previous: Option<u32>
}

#[derive(Debug, Deserialize, Serialize)]
struct TokenData {
    ta: bool,
    table_number: u8,
    name: String,
    task: String,
    helped_by: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct Claims {
    sub: String,
    exp: usize
}

#[derive(Debug, Deserialize, Serialize)]
struct Output {
    error: String,
    data: String,
    message_type: OutType
}

#[derive(Debug, Deserialize, Serialize)]
struct StudentCommand<'a> {
    command: &'a str,
}

#[derive(Debug, Deserialize, Serialize)]
struct TaCommand<'a> {
    command: &'a str,
    argument: u8
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

enum QueueState{
    Q(QueueElement),
    H(QueueElement),
    N
}

enum ThreadCommand{
    Shut,
    Send(WarpMessage)
}

type Students = Arc<Mutex<[Option<mpsc::UnboundedSender<ThreadCommand>>; 31]>>;
type TAs = Arc<Mutex<Vec<mpsc::UnboundedSender<ThreadCommand>>>>;
type Queue = Arc<Mutex<[QueueState; 31]>>;
type QueueIndex = Arc<Mutex<Option<u32>>>;

impl Claims {
    /// Attempts to parse the `sub` field into a `SubClaims` struct
    fn parse_sub(&self) -> Option<TokenData> {
        serde_json::from_str(&self.sub).ok()
    }
}

// Get JWT secret from environment or fallback to default
fn get_jwt_secret() -> String {
    env::var("JWT_PASSWORD").unwrap_or_else(|_| "supersecretkey".to_string())
}

fn validate_student_cookie(cookie: Option<String>) -> Option<TokenData> {
    let jwt_secret = get_jwt_secret();
    println!("Validating cookie with key {}", jwt_secret);
    if let Some(cookie_str) = cookie {
        let token = cookie_str.split("access_token_cookie=").nth(1)?;
        match decode::<Claims>(
            token,
            &DecodingKey::from_secret(jwt_secret.as_ref()),
            &Validation::default()
        ) {
            Ok(token_data) => {
                println!("Test1");
                let claims = token_data.claims.parse_sub();
                if let Some(token_data) = claims {
                    if (1..=30).contains(&token_data.table_number) {
                        Option::Some(token_data)
                    } else {
                        None
                    }
                }
                else {
                    println!("Test2");
                    None
                }
            }
            Err(_) => {
                println!("Cookie validation failed");
                None
            },
        }
    } else {
        None
    }
}

fn validate_ta_cookie(cookie: Option<String>) -> Option<TokenData> {
    let jwt_secret = get_jwt_secret();
    if let Some(cookie_str) = cookie {
        let token = cookie_str.split("access_token_cookie=").nth(1)?;
        match decode::<Claims>(
            token,
            &DecodingKey::from_secret(jwt_secret.as_ref()),
            &Validation::default()
        ) {
            Ok(token_data) => {
                let claims = token_data.claims.parse_sub();
                if let Some(token_data) = claims {
                    if token_data.ta {
                        Option::Some(token_data)
                    } else {
                        None
                    }
                }
                else {
                    None
                }
            }
            Err(_) => {
                None
            },
        }
    } else {
        None
    }
}

#[tokio::main]
async fn main() {
    let students: Students = Arc::new(Mutex::new(std::array::from_fn(|_| None)));
    let teaching_assistants: TAs = Arc::new(Mutex::new(Vec::new()));
    let queue: Queue = Arc::new(Mutex::new(std::array::from_fn(|_| QueueState::N)));
    let q_head: QueueIndex = Arc::new(Mutex::new(Option::None));
    let q_tail: QueueIndex = Arc::new(Mutex::new(Option::None));
    let h_head: QueueIndex = Arc::new(Mutex::new(Option::None));
    let h_tail: QueueIndex = Arc::new(Mutex::new(Option::None));

    let students_clone = students.clone();
    let teaching_assistants_clone = teaching_assistants.clone();
    let queue_clone = queue.clone();
    let q_head_clone = q_head.clone();
    let q_tail_clone = q_tail.clone();
    let h_head_clone = h_head.clone();
    let h_tail_clone = h_tail.clone();

    println!("Starting WebSocket server...");

    let student_route = warp::path("student")
        .and(warp::ws())
        .and(warp::header::optional::<String>("cookie"))
        .and(warp::any().map(move || students_clone.clone()))
        .and(warp::any().map(move || teaching_assistants_clone.clone()))
        .and(warp::any().map(move || queue_clone.clone()))
        .and(warp::any().map(move || q_head_clone.clone()))
        .and(warp::any().map(move || q_tail_clone.clone()))
        .and(warp::any().map(move || h_head_clone.clone()))
        .and(warp::any().map(move || h_tail_clone.clone()))
        .map(|
            ws: warp::ws::Ws,
            cookie: Option<String>,
            students: Students,
            teaching_assistants: TAs,
            queue: Queue, 
            q_head: QueueIndex,
            q_tail: QueueIndex,
            h_head: QueueIndex,
            h_tail: QueueIndex,
        |{
            if let Some(cookie_value) = validate_student_cookie(cookie) {
                warp::reply::with_status(
                    ws.on_upgrade(move |socket| handle_student_websocket(
                        socket,
                        students,
                        teaching_assistants,
                        queue,
                        q_tail,
                        q_head,
                        h_head,
                        h_tail,
                        cookie_value
                    )),
                    warp::http::StatusCode::SWITCHING_PROTOCOLS
                ).into_response()
            } else {
                warp::reply::with_status("", warp::http::StatusCode::FORBIDDEN).into_response()
            }
        });

    let ta_route = warp::path("ta")
        .and(warp::ws())
        .and(warp::header::optional::<String>("cookie"))
        .and(warp::any().map(move || students.clone()))
        .and(warp::any().map(move || teaching_assistants.clone()))
        .and(warp::any().map(move || queue.clone()))
        .and(warp::any().map(move || q_head.clone()))
        .and(warp::any().map(move || q_tail.clone()))
        .and(warp::any().map(move || h_head.clone()))
        .and(warp::any().map(move || h_tail.clone()))
        .map(|
            ws: warp::ws::Ws,
            cookie: Option<String>,
            students: Students,
            teaching_assistants: TAs,
            queue: Queue, 
            q_head: QueueIndex,
            q_tail: QueueIndex,
            h_head: QueueIndex,
            h_tail: QueueIndex,
        |{
            if let Some(cookie_value) = validate_ta_cookie(cookie) {
                warp::reply::with_status(
                    ws.on_upgrade(move |socket| handle_ta_websocket(
                        socket,
                        students,
                        teaching_assistants,
                        queue,
                        q_tail,
                        q_head,
                        h_head,
                        h_tail,
                        cookie_value
                    )),
                    warp::http::StatusCode::SWITCHING_PROTOCOLS
                ).into_response()
            } else {
                warp::reply::with_status("", warp::http::StatusCode::FORBIDDEN).into_response()
            }
        });

    let routes = student_route.or(ta_route);

    warp::serve(routes)
        .run((IP, PORT))
        .await
}


async fn handle_student_websocket(
    ws: warp::ws::WebSocket,
    students: Students,
    teaching_assistants: TAs,
    queue: Queue,
    q_head: QueueIndex,
    q_tail: QueueIndex,
    h_head: QueueIndex,
    h_tail: QueueIndex,
    cookie: TokenData
){
    let (mut tx, mut rx) = ws.split();
    let (msg_tx, mut msg_rx) = mpsc::unbounded_channel();
    let table_number = cookie.table_number;
    let task = cookie.task;
    {
        let mut students_lock = students.lock().await;
        if let Some(old_msg_tx) = &students_lock[cookie.table_number as usize]{
            old_msg_tx.send(ThreadCommand::Shut);
        }
        students_lock[table_number as usize] = Option::Some(msg_tx.clone());
    }
    loop {
        tokio::select! {
            Some(result) = rx.next() => {
                if let Ok(msg) = result {
                    if msg.is_text() {
                        let msg_clone = msg.clone();
                        let student_command: Result<StudentCommand, serde_json::Error> = serde_json::from_str(msg_clone.to_str().unwrap());
                        if let Ok(command) = student_command{
                            match command.command {
                                "poll" => poll_queue(&mut tx, &msg_tx, &queue, table_number).await,
                                "join" => join_queue(&mut tx, &teaching_assistants, &queue, &q_head, &q_tail, &h_head, table_number, &task).await,
                                _ => tx.send(WarpMessage::text(serde_json::to_string(
                                    &Output{
                                        error: "Invalid command".to_string(),
                                        message_type: OutType::Error,
                                        data: "".to_string()
                                    }
                                ).unwrap())).await.unwrap(),
                            };
                        } else{
                            tx.send(WarpMessage::text("Not text")).await.unwrap();
                        }
                    }
                }
            }
            Some(info) = msg_rx.recv() => {
                if let ThreadCommand::Shut = info{
                    return;
                }
                else if let ThreadCommand::Send(data) = info{
                    let _ = tx.send(data).await;
                }
            }
        }
    }
}

async fn join_queue(
    tx: &mut SplitSink<WebSocket, WarpMessage>,
    teaching_assistants: &TAs,
    queue: &Queue,
    q_head: &QueueIndex,
    q_tail: &QueueIndex,
    h_head: &QueueIndex,
    table_number: u8,
    task: &String,
){
    let value: u32;
    {
        let ta_lock = teaching_assistants.lock().await;
        let mut queue_lock = queue.lock().await;
        let mut q_head_lock = q_head.lock().await;
        let mut q_tail_lock = q_tail.lock().await;
        if let QueueState::N = queue_lock[table_number as usize]{

        }else {
            let out = Output{
                error: "Allredy in queue".to_string(),
                message_type: OutType::Error,
                data: "".to_string()
            };
            tx.send(WarpMessage::text(serde_json::to_string(&out).unwrap())).await;
            return ;
        }
        if let Some(qt) = *q_tail_lock  {
            if let QueueState::Q(q) = &mut queue_lock[qt as usize]{
                value = q.value+1;
                q.next = Option::Some(table_number as u32);
                let qe = QueueElement{
                    value: q.value+1,
                    helped_by: "".to_string(),
                    task: task.to_string(),
                    next: None,
                    previous: Some(qt)
                };
                queue_lock[table_number as usize] = QueueState::Q(qe)
            }
            else {
                panic!("Big error")
            }
        }
        else{
            value = 1;
            let qe = QueueElement{
                value: 1,
                helped_by: "".to_string(),
                task: task.to_string(),
                next: None,
                previous: None
            };
            queue_lock[table_number as usize] = QueueState::Q(qe);
            *q_head_lock = Some(table_number as u32);
        }
        *q_tail_lock = Some(table_number as u32);
    }
    let out = Output{
        error: "".to_string(),
        message_type: OutType::Index,
        data: value.to_string()
    };

    tx.send(WarpMessage::text(serde_json::to_string(&out).unwrap())).await;
    {
        let mut ta_lock = teaching_assistants.lock().await;
        let queue_lock = queue.lock().await;
        let q_head_lock = q_head.lock().await;
        let h_head_lock = h_head.lock().await;

        let queue = build_queue(queue_lock, q_head_lock, h_head_lock);
        ta_lock.retain(|ta| {
            let out = Output{
                error: "".to_string(),
                message_type: OutType::Queue,
                data: (&queue).to_string()
            };
            let message = WarpMessage::text(serde_json::to_string(&out).unwrap());
            ta.send(ThreadCommand::Send(message)).is_ok()
        });
    }
}

async fn poll_queue(
    tx: &mut SplitSink<WebSocket, WarpMessage>,
    msg_tx: &mpsc::UnboundedSender<ThreadCommand>,
    queue: &Queue,
    table_number: u8
){
    let queue_lock = queue.lock().await;
    if let QueueState::Q(q) = &queue_lock[table_number as usize]{
        let data = format!("{}", q.value);
        let out = Output{
            error: "".to_string(),
            message_type: OutType::Index,
            data
        };
        tx.send(WarpMessage::text(serde_json::to_string(&out).unwrap())).await;
    } else if let QueueState::H(h) = &queue_lock[table_number as usize]{
        let data = format!("Getting help from {}", h.helped_by);
        let out = Output{
            error: "".to_string(),
            message_type: OutType::Helping,
            data
        };
        tx.send(WarpMessage::text(serde_json::to_string(&out).unwrap())).await;
    } else{
        let out = Output{
            error: "".to_string(),
            message_type: OutType::NotQueue,
            data: "Not in queue".to_string()
        };
        tx.send(WarpMessage::text(serde_json::to_string(&out).unwrap())).await;
        msg_tx.send(ThreadCommand::Shut);
    }
}

async fn handle_ta_websocket(
    ws: warp::ws::WebSocket,
    students: Students,
    teaching_assistants: TAs,
    queue: Queue,
    q_head: QueueIndex,
    q_tail: QueueIndex,
    h_head: QueueIndex,
    h_tail: QueueIndex,
    cookie: TokenData
){
    let (mut tx, mut rx) = ws.split();
    let (msg_tx, mut msg_rx) = mpsc::unbounded_channel();
    let name = cookie.name;
    {
        let mut ta_lock = teaching_assistants.lock().await;
        ta_lock.push(msg_tx.clone());
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
                                "help" => help_student(&mut tx, &students, &teaching_assistants, &queue, &q_head, &q_tail, &h_head, &h_tail, &name, command.argument).await,
                                "remove" => remove_student(&mut tx, &students, &teaching_assistants, &queue, &q_head, &q_tail, &h_head, &h_tail, command.argument).await,
                                "get_queue" => get_queue(&mut tx, &queue, &q_head, &h_head).await,
                                _ => tx.send(WarpMessage::text(serde_json::to_string(
                                    &Output{
                                        error: "Invalid command".to_string(),
                                        message_type: OutType::Error,
                                        data: "".to_string()
                                    }
                                ).unwrap())).await.unwrap()
                            };
                        }
                        else{
                            tx.send(WarpMessage::text("Not text")).await.unwrap();
                        }
                    }
                }
            }
            Some(info) = msg_rx.recv() => {
                if let ThreadCommand::Shut = info{
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
    students: &Students,
    teaching_assistants: &TAs,
    queue: &Queue,
    q_head: &QueueIndex,
    q_tail: &QueueIndex,
    h_head: &QueueIndex,
    h_tail: &QueueIndex,
    name: &String,
    table_number: u8,
){
    {
        let student_lock = students.lock().await;
        let mut queue_lock = queue.lock().await;
        let mut q_head_lock = q_head.lock().await;
        let mut q_tail_lock = q_tail.lock().await;
        let mut h_tail_lock = h_tail.lock().await;

        match queue_lock[table_number as usize] {
            QueueState::N => {
                let out = Output {
                    error: "Student not in queue".to_string(),
                    message_type: OutType::Error,
                    data: "".to_string(),
                };
                tx.send(WarpMessage::text(serde_json::to_string(&out).unwrap())).await.unwrap();
                return;
            }
            QueueState::H(_) => {
                let out = Output {
                    error: "Student already getting help".to_string(),
                    message_type: OutType::Error,
                    data: "".to_string(),
                };
                tx.send(WarpMessage::text(serde_json::to_string(&out).unwrap())).await.unwrap();
                return;
            }
            QueueState::Q(ref mut q) => {
                // Temporarily store indices
                let prev_idx = q.previous;
                let next_idx = q.next;

                // Update previous node if it exists
                if let Some(qi_prev) = prev_idx {
                    if let QueueState::Q(ref mut q_prev) = queue_lock[qi_prev as usize] {
                        q_prev.next = next_idx;
                    } else{
                        panic!("Error in linked list implementation")
                    }
                } else {
                    *q_head_lock = next_idx;
                }

                // Update next node if it exists
                if let Some(qi_next) = next_idx {
                    if let QueueState::Q(ref mut q_next) = queue_lock[qi_next as usize] {
                        q_next.previous = prev_idx;
                    } else{
                        panic!("Error in linked linst implementation")
                    }
                } else {
                    *q_tail_lock = prev_idx;
                }

                let mut itr_next_idx = next_idx;
                while let Some(qi_next) = itr_next_idx{
                    if let QueueState::Q(ref mut q_next) = queue_lock[qi_next as usize] {
                        q_next.value = q_next.value-1;
                        itr_next_idx = q_next.next;
                        if let Some(next_tx) = &student_lock[qi_next as usize]{
                            let data = format!("{}", q_next.value);
                            let out = Output{
                                error: "".to_string(),
                                message_type: OutType::Index,
                                data
                            };
                            let message = WarpMessage::text(serde_json::to_string(&out).unwrap());
                            next_tx.send(ThreadCommand::Send(message));
                        }
                    } else{
                        panic!("Error in linked linst implementation")
                    }
                }

                // Remove student from queue and set as helping
                let he = QueueElement {
                    value: 0,
                    helped_by: name.to_string(),
                    task: "".to_string(),
                    next: Option::None,
                    previous: *h_tail_lock,
                };

                // If there was a previous help tail, update it
                if let Some(qt) = *h_tail_lock {
                    if let QueueState::H(ref mut h) = queue_lock[qt as usize] {
                        h.next = Some(table_number as u32);
                    }
                } else {
                    *h_head.lock().await = Some(table_number as u32);
                }

                *h_tail_lock = Some(table_number as u32);
                queue_lock[table_number as usize] = QueueState::H(he);
            }
        }
    }

    let data = format!("Now helping {}", table_number);
    let out = Output {
        error: "".to_string(),
        message_type: OutType::Info,
        data
    };

    tx.send(WarpMessage::text(serde_json::to_string(&out).unwrap())).await.unwrap();
    {
        let student_lock = students.lock().await;
        let mut ta_lock = teaching_assistants.lock().await;
        let queue_lock = queue.lock().await;
        let q_head_lock = q_head.lock().await;
        let h_head_lock = h_head.lock().await;

        let queue = build_queue(queue_lock, q_head_lock, h_head_lock);
        ta_lock.retain(|ta| {
            let out = Output{
                error: "".to_string(),
                message_type: OutType::Queue,
                data: (&queue).to_string()
            };
            let message = WarpMessage::text(serde_json::to_string(&out).unwrap());
            ta.send(ThreadCommand::Send(message)).is_ok()
        });
        if let Some(helped_tx) = &student_lock[table_number as usize]{
            let data = format!("Getting help from {}", name);
            let out = Output{
                error: "".to_string(),
                message_type: OutType::Helping,
                data
            };
            let data = WarpMessage::text(serde_json::to_string(&out).unwrap());
            helped_tx.send(ThreadCommand::Send(data));
        }
    }
}

async fn remove_student(
    tx: &mut SplitSink<WebSocket, WarpMessage>,
    students: &Students,
    teaching_assistants: &TAs,
    queue: &Queue,
    q_head: &QueueIndex,
    q_tail: &QueueIndex,
    h_head: &QueueIndex,
    h_tail: &QueueIndex,
    table_number: u8
){
    {
        let student_lock = students.lock().await;
        let mut queue_lock = queue.lock().await;
        let mut q_head_lock = q_head.lock().await;
        let mut q_tail_lock = q_tail.lock().await;
        let mut h_head_lock = h_head.lock().await;
        let mut h_tail_lock = h_tail.lock().await;

        match queue_lock[table_number as usize] {
            QueueState::N => {
                let out = Output {
                    error: "Student not in queue".to_string(),
                    message_type: OutType::Error,
                    data: "".to_string(),
                };
                tx.send(WarpMessage::text(serde_json::to_string(&out).unwrap())).await.unwrap();
                return;
            }
            QueueState::H(ref mut h) => {
                // Temporarily store indices
                let prev_idx = h.previous;
                let next_idx = h.next;

                // Update previous node if it exists
                if let Some(hi_prev) = prev_idx {
                    if let QueueState::H(ref mut h_prev) = queue_lock[hi_prev as usize] {
                        h_prev.next = next_idx;
                    } else{
                        panic!("Error in linked linst implementation")
                    }
                } else {
                    *h_head_lock = next_idx;
                }

                // Update next node if it exists
                if let Some(hi_next) = next_idx {
                    if let QueueState::H(ref mut h_next) = queue_lock[hi_next as usize] {
                        h_next.previous = prev_idx;
                    } else{
                        panic!("Error in linked linst implementation")
                    }
                } else {
                    *h_tail_lock = prev_idx;
                }

                queue_lock[table_number as usize] = QueueState::N;

                let data = format!("Removing student {}", table_number);
                let out = Output {
                    error: "".to_string(),
                    message_type: OutType::Info,
                    data,
                };
                tx.send(WarpMessage::text(serde_json::to_string(&out).unwrap())).await.unwrap();
                if let Some(removed_tx) = &student_lock[table_number as usize]{
                    let out = Output{
                        error: "".to_string(),
                        message_type: OutType::NotQueue,
                        data: "Not in queue".to_string()
                    };
                    let data = WarpMessage::text(serde_json::to_string(&out).unwrap());
                    removed_tx.send(ThreadCommand::Send(data));
                    removed_tx.send(ThreadCommand::Shut);

                }
            }
            QueueState::Q(ref mut q) => {
                // Temporarily store indices
                let prev_idx = q.previous;
                let next_idx = q.next;

                // Update previous node if it exists
                if let Some(qi_prev) = prev_idx {
                    if let QueueState::Q(ref mut q_prev) = queue_lock[qi_prev as usize] {
                        q_prev.next = next_idx;
                    } else{
                        panic!("Error in linked linst implementation")
                    }
                } else {
                    *q_head_lock = next_idx;
                }

                // Update next node if it exists
                if let Some(qi_next) = next_idx {
                    if let QueueState::Q(ref mut q_next) = queue_lock[qi_next as usize] {
                        q_next.previous = prev_idx;
                    } else{
                        panic!("Error in linked linst implementation")
                    }
                } else {
                    *q_tail_lock = prev_idx;
                }

                let mut itr_next_idx = next_idx;
                while let Some(qi_next) = itr_next_idx{
                    if let QueueState::Q(ref mut q_next) = queue_lock[qi_next as usize] {
                        q_next.value = q_next.value-1;
                        itr_next_idx = q_next.next;
                        if let Some(next_tx) = &student_lock[qi_next as usize]{
                            let data = format!("{}", q_next.value);
                            let out = Output{
                                error: "".to_string(),
                                message_type: OutType::Index,
                                data
                            };
                            let message = WarpMessage::text(serde_json::to_string(&out).unwrap());
                            next_tx.send(ThreadCommand::Send(message));
                        }
                    } else{
                        panic!("Error in linked linst implementation")
                    }
                }

                queue_lock[table_number as usize] = QueueState::N;

                let data = format!("Removing student {}", table_number);
                let out = Output {
                    error: "".to_string(),
                    message_type: OutType::Info,
                    data,
                };
                tx.send(WarpMessage::text(serde_json::to_string(&out).unwrap())).await.unwrap();
                if let Some(removed_tx) = &student_lock[table_number as usize]{
                    println!("{}", table_number);
                    let out = Output{
                        error: "".to_string(),
                        message_type: OutType::NotQueue,
                        data: "Not in queue".to_string()
                    };
                    let data = WarpMessage::text(serde_json::to_string(&out).unwrap());
                    removed_tx.send(ThreadCommand::Send(data));
                    removed_tx.send(ThreadCommand::Shut);

                }
            }
        }
    }
    {
        let mut ta_lock = teaching_assistants.lock().await;
        let queue_lock = queue.lock().await;
        let q_head_lock = q_head.lock().await;
        let h_head_lock = h_head.lock().await;

        let queue = build_queue(queue_lock, q_head_lock, h_head_lock);
        ta_lock.retain(|ta| {
            let out = Output{
                error: "".to_string(),
                message_type: OutType::Queue,
                data: (&queue).to_string()
            };
            let message = WarpMessage::text(serde_json::to_string(&out).unwrap());
            ta.send(ThreadCommand::Send(message)).is_ok()
        });
    }
}


async fn get_queue(
    tx: &mut SplitSink<WebSocket, WarpMessage>,
    queue: &Queue,
    q_head: &QueueIndex,
    h_head: &QueueIndex,
){
    let queue_lock = queue.lock().await;
    let q_head_lock = q_head.lock().await;
    let h_head_lock = h_head.lock().await;
    let data = build_queue(queue_lock, q_head_lock, h_head_lock);
    let out = Output {
        error: "".to_string(),
        message_type: OutType::Queue,
        data
    };

    tx.send(WarpMessage::text(serde_json::to_string(&out).unwrap())).await.unwrap();
}

fn build_queue(
    queue_lock: MutexGuard<'_, [QueueState; 31]>,
    q_head_lock: MutexGuard<'_, Option<u32>>,
    h_head_lock: MutexGuard<'_, Option<u32>>
) -> String{
    let mut queue = "".to_string();
    let mut itr_next_idx = *h_head_lock;
    while let Some(hi_next) = itr_next_idx{
        if let QueueState::H(ref h_next) = queue_lock[hi_next as usize] {
            queue = format!(
                "{}, {{\"index\": \"{}\", \"table_number\": \"{}\",\"task\": \"{}\"}}",
                queue,
                h_next.value,
                hi_next,
                h_next.task,
            );
            itr_next_idx = h_next.next;
        } else{
            panic!("Error in linked list implementation")
        }
    }
    let mut itr_next_idx = *q_head_lock;
    while let Some(qi_next) = itr_next_idx{
        if let QueueState::Q(ref q_next) = queue_lock[qi_next as usize] {
            queue = format!(
                "{}, {{\"index\": \"{}\", \"table_number\": \"{}\",\"task\": \"{}\"}}",
                queue,
                q_next.value,
                qi_next,
                q_next.task,
            );
            itr_next_idx = q_next.next;
        } else{
            panic!("Error in linked list implementation")
        }
    }
    if queue.len() > 2{
        format!("[{}]", &queue[2..])
    }else {
        "[]".to_string()
    }
}
