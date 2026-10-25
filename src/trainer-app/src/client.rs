//! 连接游戏内插件的客户端。后台线程负责连接、断线重连、每 300ms 拉一次状态，
//! 界面线程只读快照、往通道里塞命令。

use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::protocol;

const POLL_INTERVAL: Duration = Duration::from_millis(300);
const RETRY_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Clone, Default)]
pub struct Snapshot {
    pub connected: bool,
    pub plugin_version: String,
    pub state: HashMap<String, String>,
}

/// 命令执行结果，界面拿去显示。
pub struct Notice {
    pub text: String,
    pub is_error: bool,
}

struct Command {
    line: String,
    /// 成功时要显示的提示；None 表示成功时不提示（比如拖滑条）
    success_text: Option<String>,
}

pub struct Client {
    commands: Sender<Command>,
    snapshot: Arc<Mutex<Snapshot>>,
    notices: Arc<Mutex<Vec<Notice>>>,
}

impl Client {
    pub fn start(port: u16, ctx: eframe::egui::Context) -> Self {
        let (commands, rx) = mpsc::channel();
        let snapshot = Arc::new(Mutex::new(Snapshot::default()));
        let notices = Arc::new(Mutex::new(Vec::new()));
        let worker = Worker { port, rx, snapshot: snapshot.clone(), notices: notices.clone(), ctx };
        thread::Builder::new()
            .name("trainer-client".into())
            .spawn(move || worker.run())
            .expect("无法创建通信线程");
        Client { commands, snapshot, notices }
    }

    pub fn snapshot(&self) -> Snapshot {
        self.snapshot.lock().unwrap().clone()
    }

    pub fn take_notices(&self) -> Vec<Notice> {
        std::mem::take(&mut *self.notices.lock().unwrap())
    }

    pub fn set(&self, key: &str, value: &str) {
        self.send(format!("{} {key} {value}", protocol::SET), None);
    }

    pub fn action(&self, command: &str, label: &str) {
        self.send(format!("{} {command}", protocol::DO), Some(format!("已执行：{label}")));
    }

    fn send(&self, line: String, success_text: Option<String>) {
        let _ = self.commands.send(Command { line, success_text });
    }
}

struct Worker {
    port: u16,
    rx: Receiver<Command>,
    snapshot: Arc<Mutex<Snapshot>>,
    notices: Arc<Mutex<Vec<Notice>>>,
    ctx: eframe::egui::Context,
}

impl Worker {
    fn run(self) {
        let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, self.port));
        loop {
            if let Ok(stream) = TcpStream::connect_timeout(&addr, Duration::from_millis(500)) {
                // 会话结束（断开、出错）就回到重连
                let _ = self.session(stream);
            }

            {
                let mut snap = self.snapshot.lock().unwrap();
                if snap.connected {
                    *snap = Snapshot::default();
                    self.ctx.request_repaint();
                }
            }

            // 没连上时积压的命令直接作废
            let deadline = Instant::now() + RETRY_INTERVAL;
            loop {
                match self.rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
                    Ok(_) => self.notice("未连接到游戏".into(), true),
                    Err(RecvTimeoutError::Timeout) => break,
                    Err(RecvTimeoutError::Disconnected) => return,
                }
            }
        }
    }

    fn session(&self, stream: TcpStream) -> io::Result<()> {
        stream.set_nodelay(true)?;
        stream.set_read_timeout(Some(Duration::from_secs(3)))?;
        let mut writer = stream.try_clone()?;
        let mut reader = BufReader::new(stream);

        let hello = read_line(&mut reader)?;
        let Some(version) = hello.strip_prefix(protocol::HELLO) else {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "握手失败"));
        };
        {
            let mut snap = self.snapshot.lock().unwrap();
            snap.connected = true;
            snap.plugin_version = version.trim().to_string();
        }
        self.ctx.request_repaint();

        let mut next_poll = Instant::now();
        loop {
            match self.rx.recv_timeout(next_poll.saturating_duration_since(Instant::now())) {
                Ok(command) => {
                    let reply = request(&mut writer, &mut reader, &command.line)?;
                    if let Some(error) = reply.strip_prefix(protocol::ERR) {
                        self.notice(error.trim().to_string(), true);
                    } else if let Some(text) = command.success_text {
                        self.notice(text, false);
                    }
                    // 改完立刻刷新一次状态
                    next_poll = Instant::now();
                }
                Err(RecvTimeoutError::Timeout) => {
                    let reply = request(&mut writer, &mut reader, protocol::GET)?;
                    if let Some(body) = reply.strip_prefix(protocol::STATE) {
                        self.snapshot.lock().unwrap().state = parse_state(body);
                        self.ctx.request_repaint();
                    }
                    next_poll = Instant::now() + POLL_INTERVAL;
                }
                Err(RecvTimeoutError::Disconnected) => return Ok(()),
            }
        }
    }

    fn notice(&self, text: String, is_error: bool) {
        self.notices.lock().unwrap().push(Notice { text, is_error });
        self.ctx.request_repaint();
    }
}

fn request(writer: &mut TcpStream, reader: &mut BufReader<TcpStream>, line: &str) -> io::Result<String> {
    writer.write_all(line.as_bytes())?;
    writer.write_all(b"\n")?;
    read_line(reader)
}

fn read_line(reader: &mut BufReader<TcpStream>) -> io::Result<String> {
    let mut line = String::new();
    if reader.read_line(&mut line)? == 0 {
        return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "连接已断开"));
    }
    Ok(line.trim_end_matches(['\r', '\n']).to_string())
}

fn parse_state(body: &str) -> HashMap<String, String> {
    body.split(';')
        .filter_map(|pair| pair.split_once('='))
        .map(|(k, v)| (k.trim().to_string(), v.to_string()))
        .collect()
}
