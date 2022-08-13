use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::process::id;
use std::sync::{mpsc, Arc, Mutex};
use std::thread::JoinHandle;

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(tag = "cmd", content = "params")]
pub enum Command {
    Start,
    Stop,
    Status,
    SetConfig { config: Config },
    GetConfig,
    RandomInt { min: i64, max: i64 },
    RandomFloat { min: f64, max: f64 },
    Quit,
    Bye,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(tag = "response", content = "value")]
pub enum Response {
    Hello,
    OK,
    Error {
        reason: String,
    },
    StatusNotRunning {
        uptime: u64,
    },
    StatusRunning {
        system_uptime: u64,
        thread_uptime: u64,
    },
    Config {
        config: Config,
    },
    RandomInt {
        value: i64,
    },
    RandomFloat {
        value: f64,
    },
    Steam,
    Goodbye,
    ShuttingDown
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Config {
    pub seed: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            seed: 0
        }
    }
}

pub trait Applicable {
    fn apply<F>(self: Self, func: F) -> Self
    where
        F: FnOnce(Self) -> Self,
        Self: Sized;
}

impl<T> Applicable for T {
    fn apply<F>(self: Self, func: F) -> Self
    where
        F: FnOnce(Self) -> Self,
    {
        func(self)
    }
}

/*
struct Worker {
    id: usize,
    handle: JoinHandle<()>
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Self {
        let handle = std::thread::spawn(move || {
            debug!("Started Worker {}", id);
            loop {
                let msg = receiver.lock().unwrap().recv();
                match msg {
                    Ok(job) => {
                        debug!("Worker {} got a job, executing.", id);
                        job.0();
                    }
                    Err(_) => {
                        debug!("Worker {} shutting down.", id);
                        break;
                    }
                }
            }
        });
        Self { id, handle }
    }
}

struct Job(Box<dyn FnOnce() + Send + 'static>);

pub struct ThreadPool {
    threads: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>
}

impl ThreadPool {
    /// Create a new ThreadPool.
    ///
    /// The size is the number of threads in the pool.
    ///
    /// # Panics
    ///
    /// The `new` function will panic if the size is zero.
    pub fn new(size: usize) -> Self {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();
        let sender = Some(sender);
        let receiver = Arc::new(Mutex::new(receiver));

        let mut threads = Vec::with_capacity(size);
        for i in 0..size {
            threads.push(Worker::new(i, receiver.clone()));
        }

        ThreadPool { sender, threads }
    }

    pub fn execute<F>(&self, f: F)
        where
            F: FnOnce() + Send + 'static
    {
        let job = Job(Box::new(f));
        self.sender.as_ref().unwrap().send(job).unwrap()
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        info!("Shutting down, waiting for workers to finish...");
        drop(self.sender.take().unwrap());

        for worker in self.threads.drain(0..) {
            debug!("Shutting down worker {}.", worker.id);
            worker.handle.join().expect("Unable to gracefully stop worker.");
        }
    }
}*/
