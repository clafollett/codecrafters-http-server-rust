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

        return ThreadPool {
            workers,
            sender
        };
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
    thread: JoinHandle<Arc<Mutex<Receiver<Task>>>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<Receiver<Task>>>) -> Worker {
        let thread = thread::spawn(move || loop {
            let task = receiver.lock().unwrap().recv().unwrap();

            print!("Worker {id} started a new task\n");
            task();
        });

        Worker { id, thread }
    }
}

type Task = Box<dyn FnOnce() + Send + 'static>;