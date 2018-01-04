use std::thread;
use std::sync::{Arc, Mutex, RwLock};
use std::net::{IpAddr, SocketAddr};
use std::io::Result;
use mio::*;
use mio::net::{TcpListener, TcpStream};
use eventloop::*;
use channel::*;


pub struct Acceptor {
    host: String,
    port: u16,
    eventloop_group: Option<Arc<Vec<EventLoop>>>,
    func: Option<fn(&mut Channel) -> Result<()>>,
    eventloop_count: usize,
}

impl Acceptor {
    pub fn new() -> Acceptor {
        Acceptor {
            host: "0.0.0.0".to_owned(),
            port: 12345,
            eventloop_group: None,
            func: None,
            eventloop_count: 0,
        }
    }

    pub fn worker_count(&mut self, size: usize) -> &mut Acceptor {
        self.eventloop_count = size;
        let mut group = Vec::<EventLoop>::new();
        for _i in 0..size {
            group.push(EventLoop::new());
        }
        self.eventloop_group = Some(Arc::new(group));
        self
    }

    pub fn handler(&mut self, f: fn(&mut Channel) -> Result<()>) -> &mut Acceptor {
        self.func = Some(f);
        self
    }

    pub fn bind(&mut self, host: &str, port: u16) -> &mut Acceptor {
        self.host = host.to_string();
        self.port = port;
        self
    }

    pub fn shutdown(&self) {
        //need ref of eventloop_group
    }

    pub fn accept(&self) {
        let group = match &self.eventloop_group {
            None => panic!(""),
            Some(g) => Arc::clone(&g),
        };
        let ip_addr = self.host.parse().unwrap();
        let sock_addr = Arc::new(SocketAddr::new(ip_addr, self.port));
        let count = Arc::new(self.eventloop_count);
        let const_count = Arc::clone(&count);
        //FIXME magic?
        let f = self.func;
        thread::spawn(move || {
            let mut events = Events::with_capacity(1024);
            let mut ch_id: usize = 0;
            let listener = TcpListener::bind(&sock_addr).unwrap();
            let selector = Poll::new().unwrap();
            selector
                .register(&listener, Token(0), Ready::readable(), PollOpt::edge())
                .unwrap();
            //TODO processor * 2
            let const_count = Arc::try_unwrap(const_count).unwrap_or(1);
            for eventloop in group.iter() {
                eventloop.run();
            }
            loop {
                match selector.poll(&mut events, None) {
                    Ok(_) => {}
                    Err(_) => {}
                }
                for _e in events.iter() {
                    let (mut sock, addr) = match listener.accept() {
                        Ok((s, a)) => (s, a),
                        Err(_) => {
                            continue;
                        }
                    };
                    group[ch_id % const_count].attach(&mut sock, &addr, Token(ch_id), f);
                    ch_id = Acceptor::incr_id(ch_id);
                }
            }
        });
    }

    #[inline]
    fn incr_id(cur_id: usize) -> usize {
        if cur_id >= usize::max_value() {
            0
        } else {
            cur_id + 1
        }
    }
}
