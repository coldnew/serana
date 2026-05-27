use std::sync::Arc;
use std::thread;

use parking_lot::Mutex;

pub struct ThreadPool {
    workers: Vec<thread::JoinHandle<()>>,
    sender: Option<channel::Sender<Box<dyn FnOnce() + Send>>>,
}

mod channel {
    use parking_lot::{Condvar, Mutex};
    use std::collections::VecDeque;
    use std::sync::Arc;

    struct Shared<T> {
        queue: Mutex<VecDeque<T>>,
        condvar: Condvar,
        closed: Mutex<bool>,
    }

    pub struct Sender<T> {
        shared: Arc<Shared<T>>,
    }

    impl<T> Clone for Sender<T> {
        fn clone(&self) -> Self {
            Self {
                shared: self.shared.clone(),
            }
        }
    }

    impl<T> Sender<T> {
        pub fn send(&self, msg: T) {
            let mut queue = self.shared.queue.lock();
            queue.push_back(msg);
            drop(queue);
            self.shared.condvar.notify_one();
        }
    }

    impl<T> Drop for Sender<T> {
        fn drop(&mut self) {
            *self.shared.closed.lock() = true;
            self.shared.condvar.notify_all();
        }
    }

    pub struct Receiver<T> {
        shared: Arc<Shared<T>>,
    }

    impl<T> Receiver<T> {
        pub fn recv(&self) -> Option<T> {
            let mut queue = self.shared.queue.lock();
            loop {
                if let Some(msg) = queue.pop_front() {
                    return Some(msg);
                }
                if *self.shared.closed.lock() {
                    return None;
                }
                self.shared.condvar.wait(&mut queue);
            }
        }
    }

    pub fn unbounded<T>() -> (Sender<T>, Receiver<T>) {
        let shared = Arc::new(Shared {
            queue: Mutex::new(VecDeque::new()),
            condvar: Condvar::new(),
            closed: Mutex::new(false),
        });
        (
            Sender { shared: shared.clone() },
            Receiver { shared },
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
                        None => break,
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
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}
