use std::{
    sync::{mpsc::{self, Receiver}, Arc, Mutex},
    thread::{self, JoinHandle}
};

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Task>
}

impl ThreadPool {
    pub fn new (size: usize) -> ThreadPool{
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));
        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        let pool = ThreadPool {
            workers,
            sender
        };

        print!("ThreadPool created: size={}\n", pool.workers.len());

        return pool;
    }

    pub fn queue<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let task = Box::new(f);
        self.sender.send(task).unwrap();
    }
}

struct Worker {
    id: usize,
    handle: JoinHandle<Arc<Mutex<Receiver<Task>>>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<Receiver<Task>>>) -> Worker {
        let thread_name = format!("Thread-{}", id);
        let handle = thread::Builder::new().name(thread_name).spawn(move || loop {
            let task = receiver.lock().unwrap().recv().unwrap();
            task();
        }).unwrap();

        let worker = Worker { id, handle };

        println!("Worker-{}:{} created", worker.id, worker.handle.thread().name().unwrap());

        return worker;
    }
}

type Task = Box<dyn FnOnce() + Send + 'static>;