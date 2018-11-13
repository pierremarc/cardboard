use std::cmp::{Ord, Ordering};
use std::collections::BinaryHeap;
use std::collections::VecDeque;
use std::slice::Iter;
use std::sync::mpsc::channel;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::sync::{Arc, RwLock};

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

struct Worker<I: Copy, O: Copy> {
    command: Receiver<Indexed<I>>,
    result: Sender<Indexed<O>>,
}

impl<I: Copy, O: Copy> Worker<I, O> {
    fn run<F>(&mut self, mut f: F)
    where
        F: FnMut(I) -> O,
    {
        for input in self.command.iter() {
            let i = input.index();
            let r = f(input.get());
            self.result.send(Indexed::new(i, r)).unwrap_or(())
        }
    }
}

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

pub struct Paro<'a, S: 'static + Copy + Send + Sync, T: 'static + Copy + Send> {
    source: Iter<'a, S>,
    buffer: RwLock<Arc<BinaryHeap<Indexed<T>>>>,
    cms: Vec<Commander<S>>,
    send_result: Sender<Indexed<T>>,
    rec_result: Receiver<Indexed<T>>,
    head: usize,
    tail: RwLock<Arc<usize>>,
    sent: usize,
}

impl<'a, S: 'static + Copy + Send + Sync, T: 'static + Copy + Send> Paro<'a, S, T> {
    pub fn new(source: Iter<'a, S>) -> Paro<S, T> {
        let (tx_r, rx_r) = channel::<Indexed<T>>();
        Paro {
            source,
            cms: Vec::new(),
            buffer: RwLock::new(Arc::new(BinaryHeap::new())),
            rec_result: rx_r,
            send_result: tx_r,
            head: ::std::usize::MAX,
            tail: RwLock::new(Arc::new(::std::usize::MAX)),
            sent: 0,
        }
    }

    pub fn start(&mut self) {
        let r = self.rec_result;
        let mut b = self.buffer;
        let mut t = self.tail;
        thread::spawn(move || {
            for res in r.recv() {
                match b.write() {
                    Ok(g)  =>{
                        *g.
                    },
                    Err(_) => ()
                }
                // self.buffer.push(res);
                // self.tail -= 1;
            }
        });
    }

    pub fn add_worker<F>(&mut self, f: F)
    where
        F: FnMut(S) -> T + Send + 'static,
    {
        let (tx_c, rx_c) = channel::<Indexed<S>>();

        let mut w = Worker {
            command: rx_c,
            result: self.send_result.clone(),
        };

        self.cms.push(Commander(tx_c));

        thread::spawn(move || w.run(f));
        self.buffer.reserve(1);
    }

    fn prefetch(&mut self) {
        let len = self.cms.len();
        for i in 0..len {
            let args = self.source.next();
            let com = &self.cms[i];
            match args {
                Some(a) => {
                    self.head -= 1;
                    com.send(self.head, *a);
                }
                None => (),
            };
        }
    }

    pub fn next(&mut self) -> Next<T> {
        // let mut ready = false;
        {
            match self.buffer.peek() {
                Some(val) => {
                    if val.index() != self.tail {
                        return Next::Unordered;
                    }
                }
                None => {
                    if self.head == self.tail {
                        return Next::End;
                    } else {
                        return Next::NotReady;
                    }
                }
            }
        }

        match self.buffer.pop() {
            Some(val) => Next::Some(val.get()),
            None => Next::Error,
        }
    }
}
