use std::io::{BufRead, BufReader, BufWriter, Write};
use std::net::{SocketAddr, TcpStream};
use std::str::FromStr;
use std::time::Duration;
use common::{Command, Config, Response};

#[derive(Debug)]
enum ClientError {
    IO(std::io::Error),
    JSON(serde_json::Error),
    Server(String),
    Client(String),
    Unknown(String)
}

impl From<std::io::Error> for ClientError {
    fn from(it: std::io::Error) -> Self {
        Self::IO(it)
    }
}

impl From<serde_json::Error> for ClientError {
    fn from(it: serde_json::Error) -> Self {
        Self::JSON(it)
    }
}

impl From<String> for ClientError {
    fn from(it: String) -> Self {
        Self::Unknown(it)
    }
}

type Result<T> = core::result::Result<T, ClientError>;

struct Status {
    running: bool,
    system_uptime: u64,
    thread_uptime: Option<u64>
}

struct Client {
    addr: SocketAddr,
    sock: TcpStream
}

impl Client {
    fn new(addr: &str) -> Result<Self> {
        let addr = SocketAddr::from_str(addr).map_err(|e| {
            ClientError::Client(format!("Failed to parse address: {:?}", e))
        })?;
        eprintln!("[.] Connecting to {}", addr);
        let sock = TcpStream::connect_timeout(&addr, Duration::from_secs(5))?;
        let client = Self { addr, sock };

        // Wait for server to say Hello
        if let Response::Hello = client.recv_once()? {
            eprintln!("[*] Successfully connected to {}", addr);
            Ok(client)
        } else {
            Err(ClientError::Server("Server did not say hello!".into()))
        }
    }

    fn reader(&self) -> BufReader<&TcpStream> {
        BufReader::new(&self.sock)
    }

    fn writer(&self) -> BufWriter<&TcpStream> {
        BufWriter::new(&self.sock)
    }

    fn send(&self, cmd: &Command) -> Result<()> {
        let mut write = self.writer();
        eprintln!("[.] Sending command {:?}", &cmd);
        serde_json::to_writer(&mut write, &cmd)?;
        write.write(b"\n")?;
        write.flush()?;
        Ok(())
    }

    fn recv_once(&self) -> Result<Response> {
        eprintln!("[.] Waiting for response... ");
        let mut read = self.reader();
        let mut resp_json = String::new();
        read.read_line(&mut resp_json)?;
        let resp = serde_json::from_str(&resp_json)?;
        eprintln!("[.] Server responded {:?}", resp);
        Ok(resp)
    }

    fn req_ok(&self, cmd: &Command) -> Result<()> {
        self.send(cmd)?;
        match self.recv_once()? {
            Response::OK => Ok(()),
            Response::Error { reason } => Err(ClientError::Client(reason)),
            other => Err(ClientError::Server(format!("Unexpected response from server: {:?}", other)))
        }
    }

    fn req_get_config(&self) -> Result<Config> {
        self.send(&Command::GetConfig)?;
        match self.recv_once()? {
            Response::Config { config } => Ok(config),
            Response::Error { reason } => Err(ClientError::Client(reason)),
            other => Err(ClientError::Server(format!("Unexpected response from server: {:?}", other)))
        }
    }

    fn req_set_config(&self, config: Config) -> Result<()> {
        self.send(&Command::SetConfig { config })?;
        match self.recv_once()? {
            Response::Config { config } => Ok(()),
            Response::Error { reason } => Err(ClientError::Client(reason)),
            other => Err(ClientError::Server(format!("Unexpected response from server: {:?}", other)))
        }
    }

    fn req_status(&self) -> Result<Status> {
        self.send(&Command::Status)?;
       match self.recv_once()? {
           Response::StatusRunning { system_uptime, thread_uptime } => Ok(Status {
               system_uptime,
               thread_uptime: Some(thread_uptime),
               running: true
           }),
           Response::StatusNotRunning { uptime } => Ok(Status {
               system_uptime: uptime,
               thread_uptime: None,
               running: false
           }),
           other => Err(ClientError::Server(format!("Unexpected response from server: {:?}", other)))
       }
    }

    fn req_bye(&self) -> Result<()> {
        eprintln!("[.] Sending disconnect to {}", self.addr);
        self.send(&Command::Bye)?;
        match self.recv_once()? {
            Response::Goodbye => Ok(()),
            Response::Error { reason } => Err(ClientError::Client(reason)),
            other => Err(ClientError::Server(format!("Unexpected response from server: {:?}", other)))
        }
    }
}

fn main() {
    let address = "127.0.0.1:7878";
    let client = Client::new(address).unwrap();
    let status = client.req_status().unwrap();

    if status.running {
        // Stop server if it is running
        client.req_ok(&Command::Stop).unwrap();
    }

    client.req_set_config(Config {
        seed: 22
    }).unwrap();

    client.req_ok(&Command::Start).unwrap();
    client.req_bye().unwrap();
}
