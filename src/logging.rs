use std::fmt::Write;
use std::os::unix::net::UnixDatagram;

use anyhow::Result;
use tracing::field::{Field, Visit};
use tracing::{Event, Level};
use tracing_subscriber::layer::{Context, Layer, SubscriberExt};
use tracing_subscriber::registry::Registry;

pub fn setup_syslog() {
    let subscriber = Registry::default().with(SyslogWriterLayer::new(Level::DEBUG).unwrap());
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
}

struct SyslogWriterLayer {
    max_level: Level,
    socket: UnixDatagram,
}

impl SyslogWriterLayer {
    pub fn new(max_level: Level) -> Result<Self> {
        let socket = UnixDatagram::unbound()?;
        socket.connect("/dev/log")?;
        Ok(Self { max_level, socket })
    }
}

impl<S> Layer<S> for SyslogWriterLayer
where
    S: tracing::Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let meta = event.metadata();
        let level = meta.level();
        if level > &self.max_level {
            return;
        };
        let mut message = String::new();
        event.record(&mut StringVisitor {
            string: &mut message,
        });
        let prival = match *level {
            // Values observed on a Kobo with e.g. (for debug) `strace logger -t wifiremote -p 7 test`
            // Not sure if they actually affect anything though...
            Level::DEBUG | Level::TRACE => 15,
            Level::INFO => 14,
            Level::WARN => 12,
            Level::ERROR => 11,
        };
        let tag = format!("wifiremote[{}]", std::process::id());
        // trim the crate name from the module path
        let mpath = meta.module_path().unwrap();
        let i = mpath.find(':').unwrap() + 2;
        let module = &mpath[i..];
        let src_line = &meta.line().unwrap();
        let prefix = format!("<{prival}>{tag} {level}:");
        // Print one line at a time so the output is nicely formatted
        let mut iter = message.split('\n').peekable();
        while let Some(line) = iter.next() {
            let log = if iter.peek().is_some() {
                format!("{prefix} {line}")
            } else {
                format!("{prefix} {line} ({module}:{src_line})")
            };
            match self.socket.send(&log.into_bytes()) {
                Ok(_) => (),
                Err(e) => {
                    eprintln!("{}", message);
                    eprintln!("{e}");
                }
            }
        }
    }
}

pub struct StringVisitor<'a> {
    string: &'a mut String,
}

impl<'a> Visit for StringVisitor<'a> {
    fn record_debug(&mut self, _: &Field, value: &dyn std::fmt::Debug) {
        write!(self.string, "{:?}", value).unwrap();
    }
}
