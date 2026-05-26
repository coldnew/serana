use std::sync::Arc;
use std::thread;

use parking_lot::Mutex;

pub struct ThreadPool {
    workers: Vec<thread::JoinHandle<()>>,
    sender: Option<channel::Sender<Box<dyn FnOnce() + Send>>>,
}

// Simple channel implementation without crossbeam
mod channel {
    use parking_lot::Mutex;
    use std::collections::VecDeque;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    pub struct Sender<T> {
        queue: Arc<Mutex<VecDeque<T>>>,
        available: Arc<AtomicBool>,
    }

    impl<T> Clone for Sender<T> {
        fn clone(&self) -> Self {
            Self {
                queue: self.queue.clone(),
                available: self.available.clone(),
            }
        }
    }

    impl<T> Sender<T> {
        pub fn send(&self, msg: T) {
            self.queue.lock().push_back(msg);
            self.available.store(true, Ordering::SeqCst);
        }
    }

    pub struct Receiver<T> {
        queue: Arc<Mutex<VecDeque<T>>>,
        available: Arc<AtomicBool>,
    }

    impl<T> Receiver<T> {
        pub fn recv(&self) -> Option<T> {
            loop {
                let msg = self.queue.lock().pop_front();
                if msg.is_some() {
                    if self.queue.lock().is_empty() {
                        self.available.store(false, Ordering::SeqCst);
                    }
                    return msg;
                }
                if !self.available.load(Ordering::SeqCst) {
                    return None;
                }
                std::thread::yield_now();
            }
        }
    }

    pub fn unbounded<T>() -> (Sender<T>, Receiver<T>) {
        let queue = Arc::new(Mutex::new(VecDeque::new()));
        let available = Arc::new(AtomicBool::new(false));
        (
            Sender {
                queue: queue.clone(),
                available: available.clone(),
            },
            Receiver { queue, available },
        )
    }
}

type Job = Box<dyn FnOnce() + Send>;

impl ThreadPool {
    pub fn new(size: usize) -> Self {
        let (sender, receiver) = channel::unbounded::<Job>();
        let receiver = Arc::new(Mutex::new(receiver));
        let mut workers = Vec::with_capacity(size);

        for _id in 0..size {
            let receiver = receiver.clone();
            let handle = thread::spawn(move || {
                loop {
                    let job = {
                        let rx = receiver.lock();
                        rx.recv()
                    };
                    match job {
                        Some(job) => job(),
                        None => {
                            thread::yield_now();
                        }
                    }
                }
            });
            workers.push(handle);
        }

        Self {
            workers,
            sender: Some(sender),
        }
    }

    pub fn spawn<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        if let Some(sender) = &self.sender {
            sender.send(Box::new(f));
        }
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        drop(self.sender.take());
    }
}
