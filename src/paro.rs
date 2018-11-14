use std::cmp::{Ord, Ordering};
use std::collections::BinaryHeap;
use std::collections::VecDeque;
use std::slice::Iter;
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
use std::sync::mpsc::channel;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;

struct Indexed<T: Copy>(usize, T);

impl<T: Copy> PartialEq for Indexed<T> {
    fn eq(&self, other: &Indexed<T>) -> bool {
        self.0 == other.0
    }
}

impl<T: Copy> Eq for Indexed<T> {}

impl<T: Copy> PartialOrd for Indexed<T> {
    fn partial_cmp(&self, other: &Indexed<T>) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Copy> Ord for Indexed<T> {
    fn cmp(&self, other: &Indexed<T>) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl<T: Copy> Indexed<T> {
    fn new(i: usize, a: T) -> Indexed<T> {
        Indexed(i, a)
    }

    fn index(&self) -> usize {
        self.0
    }

    fn get(&self) -> T {
        self.1
    }
}

struct Worker<I: Copy, O: Copy, C: Copy> {
    context: Arc<C>,
    command: Receiver<Indexed<I>>,
    result_bucket: Arc<Mutex<ResultBucket<O>>>,
}

impl<I: Copy, O: Copy, C: Copy> Worker<I, O, C> {
    fn run<F>(&mut self, mut f: F)
    where
        F: FnMut(C, I) -> O,
    {
        let context = *self.context;
        for input in self.command.iter() {
            let i = input.index();
            let r = f(context, input.get());
            match self.result_bucket.lock() {
                Ok(mut bucket) => (*bucket).push(Indexed(i, r)),
                Err(_) => (),
            }
        }
    }
}

#[derive(Clone)]
struct Commander<I: Copy>(Sender<Indexed<I>>);

impl<I: Copy> Commander<I> {
    fn send(&self, i: usize, input: I) {
        self.0.send(Indexed::new(i, input)).unwrap();
    }
}

pub enum Next<T> {
    End,
    NotReady,
    Unordered,
    Error,
    Some(T),
}

struct ResultBucket<T>
where
    T: Copy,
{
    results: BinaryHeap<Indexed<T>>,
    head: usize,
}

impl<T> ResultBucket<T>
where
    T: Copy,
{
    fn new() -> ResultBucket<T> {
        ResultBucket {
            results: BinaryHeap::new(),
            head: ::std::usize::MAX,
        }
    }

    fn push(&mut self, result: Indexed<T>) {
        let i = result.index();
        self.head = ::std::cmp::min(i, self.head);
        self.results.push(result);
    }

    fn get_tail(&mut self, tail: usize) -> Option<T> {
        let mut ready = false;
        {
            ready = self
                .results
                .peek_mut()
                .map(|v| v.index() == tail)
                .unwrap_or(false);
        }

        if ready {
            self.results.pop().and_then(|v| Some(v.get()))
        } else {
            None
        }
    }
}

pub struct Paro<S, T, C>
where
    S: Copy + Send,
    T: Copy + Send,
    C: Copy + Send,
{
    source: Arc<Mutex<Vec<S>>>,
    context: C,
    cms: Arc<Mutex<Vec<Commander<S>>>>,
    tail: usize,
    head: Arc<AtomicUsize>,
    sent: usize,
    result_bucket: Arc<Mutex<ResultBucket<T>>>,
}

impl<S, T, C> Paro<S, T, C>
where
    S: Copy + Send + Sync + 'static,
    T: Copy + Send + Sync + 'static,
    C: Copy + Send + Sync + 'static,
{
    pub fn new(source: Arc<Mutex<Vec<S>>>, context: C) -> Paro<S, T, C> {
        Paro {
            source: source.clone(),
            context: context,
            cms: Arc::new(Mutex::new(Vec::new())),
            tail: ::std::usize::MAX,
            head: Arc::new(AtomicUsize::new(::std::usize::MAX)),
            sent: 0,
            result_bucket: Arc::new(Mutex::new(ResultBucket::new())),
        }
    }

    pub fn add_worker<F>(&mut self, f: F)
    where
        F: FnMut(C, S) -> T + Send + 'static,
    {
        let (tx, rx) = channel();
        let mut w = Worker {
            context: Arc::new(self.context),
            command: rx,
            result_bucket: self.result_bucket.clone(),
        };

        let mut locked_commands = self.cms.lock().unwrap();
        locked_commands.push(Commander(tx));

        thread::spawn(move || w.run(f));
    }

    fn prefetch(&mut self) {
        let mut locked_commands = self.cms.lock().unwrap();
        let commands: Vec<Commander<S>> = locked_commands.iter().map(|c| c.clone()).collect();
        let source = self.source.clone();
        let head = self.head.clone();
        let h = thread::spawn(move || {
            let mut ci = 0;
            let len = commands.len();
            let locked_source = source.lock().unwrap();
            for (i, args) in locked_source.iter().enumerate() {
                if ci == len {
                    ci = 0;
                }
                let com = &commands[ci];
                com.send(::std::usize::MAX - i, *args);
                head.fetch_sub(1, AtomicOrdering::SeqCst);
            }
        });
    }

    fn pri_next(&self) -> Result<Option<T>, usize> {
        let mut bucket = self.result_bucket.lock().unwrap();
        match bucket.get_tail(self.tail) {
            None => {
                let head = self.head.load(AtomicOrdering::Relaxed);
                if self.tail == head {
                    Ok(None)
                } else {
                    Err(self.tail - head)
                }
            }
            Some(v) => Ok(Some(v)),
        }
    }

    pub fn next(&mut self) -> Option<T> {
        loop {
            match self.pri_next() {
                Ok(v) => {
                    self.tail -= 1;
                    return v;
                }
                Err(u) => {
                    thread::sleep(::std::time::Duration::from_millis(u as u64));
                }
            }
        }
    }
}
