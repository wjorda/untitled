use common::Applicable;
use common::{Command, Config, Response};
use log::{debug, error, info, trace, warn, Level};
use std::borrow::{Borrow, Cow};
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::Receiver;
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{sleep, JoinHandle};
use std::time::{Duration, SystemTime};
use threadpool::ThreadPool;

mod thread;

struct Task {
    thread: JoinHandle<()>,
    timestamp: SystemTime,
    tx: mpsc::Sender<u8>,
}

impl Task {
    fn new(config: Config) -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            tx,
            thread: std::thread::spawn(move || thread::process(rx, config)),
            timestamp: SystemTime::now(),
        }
    }

    fn kill(self) {
        self.tx.send(1).unwrap();
        self.thread.join().unwrap();
    }
}

struct ServerState {
    config: Config,
    start_time: SystemTime,
    task: Option<Task>,
}

impl ServerState {
    fn new(config: Config) -> Self {
        Self {
            config,
            start_time: SystemTime::now(),
            task: None,
        }
    }

    fn is_running(&self) -> bool {
        self.task.is_some()
    }

    fn start(&mut self) -> Result<(), &'static str> {
        if self.is_running() {
            Err("Thread is already running.")
        } else {
            let cfg = self.config.clone();
            self.task = Some(Task::new(cfg));
            Ok(())
        }
    }

    fn stop(&mut self) -> Result<(), &'static str> {
        if self.is_running() {
            self.task.take().unwrap().kill();
            Ok(())
        } else {
            Err("Thread is not running.")
        }
    }

    fn set_config(&mut self, config: &Config) -> Result<(), &'static str> {
        if !self.is_running() {
            self.config = config.clone();
            Ok(())
        } else {
            Err("Must be stopped to change config.")
        }
    }
}

fn respond(cmd: Command, state: Arc<Mutex<ServerState>>) -> Response {
    match cmd {
        Command::Start => {
            let mut state = state.lock().unwrap();
            match state.start() {
                Err(e) => Response::Error {
                    reason: e.to_string()
                },
                Ok(_) => Response::OK
            }
        }
        Command::Stop => {
            let mut state = state.lock().unwrap();
            match state.stop() {
                Err(e) => Response::Error {
                    reason: e.to_string(),
                },
                Ok(_) => Response::OK,
            }
        }
        Command::SetConfig { config } => {
            let mut state = state.lock().unwrap();
            match state.set_config(&config) {
                Err(e) => Response::Error {
                    reason: e.to_string(),
                },
                Ok(_) => Response::Config { config },
            }
        }
        Command::GetConfig => {
            let state = state.lock().unwrap();
            Response::Config {
                config: state.config.clone(),
            }
        }
        Command::Status => {
            let state = state.lock().unwrap();
            let running = state.task.is_some();
            let uptime = state.start_time.elapsed().unwrap().as_secs();
            if running {
                let task = state.task.as_ref().unwrap();
                let thread_uptime = task.timestamp.elapsed().unwrap().as_secs();
                Response::StatusRunning {
                    thread_uptime,
                    system_uptime: uptime,
                }
            } else {
                Response::StatusNotRunning { uptime }
            }
        }
        Command::Bye => Response::Goodbye,
        _ => Response::Error {
            reason: "Command not yet implemented.".to_string(),
        },
    }
}

fn handle(mut conn: TcpStream, state: Arc<Mutex<ServerState>>) -> std::io::Result<()> {
    let mut reader = BufReader::new(&conn);
    let mut writer = BufWriter::new(&conn);

    // Say hello
    serde_json::to_writer(&mut writer, &Response::Hello)?;
    writer.write(b"\n");
    writer.flush()?;

    for line in reader.lines() {
        let mut hangup = false;
        if let Ok(request) = line {
            if let Ok(cmd) = serde_json::from_str::<Command>(&request) {
                debug!("Request: {:?}", &cmd);
                let response = respond(cmd, state.clone());
                debug!("Response: {:?}", &response);
                serde_json::to_writer(&mut writer, &response)?;
                if let Response::Goodbye = response {
                    hangup = true;
                }
            } else {
                let err = Response::Error {
                    reason: "Malformed request.".to_string(),
                };
                debug!("Response: {:?}", err);
                serde_json::to_writer(&mut writer, &err)?;
            }

            writer.write(b"\n");
            writer.flush()?;
        }

        if hangup {
            break;
        }
    }

    Ok(())
}

fn logging(level: log::Level) {
    simple_logger::init_with_level(level).expect("Failed to configure logging.");
    info!("Initialized logging!");
}

fn threading(n_threads: Option<usize>) -> ThreadPool {
    threadpool::Builder::new()
        .apply(|b| {
            if let Some(n) = n_threads {
                info!("Starting thread pool with {} threads", n);
                b.num_threads(n)
            } else {
                info!("Starting thread pool with default threads");
                b
            }
        })
        .thread_name("ThreadPool".into())
        .build()
    /*
    let n = match n_threads {
        Some(n) => n,
        None => 4
    };
    ThreadPool::new(n)
     */
}

fn main() {
    logging(Level::Debug);

    let addr = "127.0.0.1:7878";
    let listener = TcpListener::bind(addr).unwrap();
    info!("Started server on {}!", addr);

    let pool = threading(None);
    let state = Arc::new(Mutex::new(ServerState::new(Config::default())));

    for conn_res in listener.incoming() {
        let conn = conn_res.unwrap();
        let peer = conn.peer_addr().unwrap();
        debug!("Incoming connection from {}", peer);
        let state = state.clone();
        pool.execute(move || {
            let res = handle(conn, state);
            if let Err(e) = res {
                error!("Error handling connection: {:?}", e);
            }
            debug!("Done with connection from {}", peer);
        });
    }
}
