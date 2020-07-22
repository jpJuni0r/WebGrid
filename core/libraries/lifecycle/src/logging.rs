use log::info;
use redis::{aio::ConnectionLike, cmd, AsyncCommands, RedisResult};
use std::fmt;

pub struct Logger<C: ConnectionLike> {
    con: C,
    component: String,
}

// Initializer
impl<C: ConnectionLike> Logger<C> {
    pub fn new(con: C, component: String) -> Logger<C> {
        Logger { con, component }
    }
}

// Logging functions
impl<C: ConnectionLike + AsyncCommands> Logger<C> {
    #[rustfmt::skip]
    async fn log_raw(
        &mut self,
        session_id: &str,
        level: LogLevel,
        code: String,
        meta: Option<String>,
    ) -> RedisResult<()> {
        let key = format!("session:{}:log", session_id);
        let metrics_key = format!("metrics:sessions:log:{:?}", level);

        info!("Writing log code {} for {}", code, session_id);
        self.con.hincr::<_, _, _, ()>(metrics_key, &code, 1).await.ok();

        cmd("XADD")
            .arg(key).arg("*")
            .arg("component").arg(&self.component)
            .arg("level").arg(level.to_string())
            .arg("code").arg(code)
            .arg("meta").arg(meta.unwrap_or_else(|| "{}".to_string()))
            .query_async(&mut self.con)
            .await
    }

    pub async fn log(
        &mut self,
        session_id: &str,
        code: LogCode,
        meta: Option<String>,
    ) -> RedisResult<()> {
        self.log_raw(session_id, code.level(), code.to_string(), meta)
            .await
    }
}

pub struct SessionLogger<C: ConnectionLike + AsyncCommands> {
    logger: Logger<C>,
    session_id: String,
}

impl<C: ConnectionLike + AsyncCommands> SessionLogger<C> {
    pub fn new(con: C, component: String, session_id: String) -> SessionLogger<C> {
        SessionLogger {
            logger: Logger::new(con, component),
            session_id,
        }
    }

    pub async fn log(&mut self, code: LogCode, meta: Option<String>) -> RedisResult<()> {
        self.logger
            .log_raw(&self.session_id, code.level(), code.to_string(), meta)
            .await
    }
}

#[derive(Debug)]
pub enum LogLevel {
    INFO,
    WARN,
    FAIL,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug)]
pub enum LogCode {
    // Generic
    // -- Fail
    FAILURE,

    // Node
    // -- Info
    BOOT,
    DSTART,
    DALIVE,
    LSINIT,
    CLOSED,
    HALT,
    // -- Fail
    DTIMEOUT,
    DFAILURE,
    STIMEOUT,
    TERM,

    // Orchestrator
    // -- Info
    SCHED,
    // -- Fail
    STARTFAIL,

    // Manager
    // -- Info
    QUEUED,
    NALLOC,
    PENDING,
    NALIVE,
    // -- Warn
    CLEFT,
    // -- Fail
    INVALIDCAP,
    QUNAVAILABLE,
    QTIMEOUT,
    OTIMEOUT,
    NTIMEOUT,
    // Proxy
}

impl LogCode {
    pub fn level(&self) -> LogLevel {
        match self {
            // Generic
            LogCode::FAILURE => LogLevel::FAIL,

            // Node
            LogCode::BOOT => LogLevel::INFO,
            LogCode::DSTART => LogLevel::INFO,
            LogCode::DALIVE => LogLevel::INFO,
            LogCode::LSINIT => LogLevel::INFO,
            LogCode::CLOSED => LogLevel::INFO,
            LogCode::HALT => LogLevel::INFO,

            LogCode::DTIMEOUT => LogLevel::FAIL,
            LogCode::DFAILURE => LogLevel::FAIL,
            LogCode::STIMEOUT => LogLevel::FAIL,
            LogCode::TERM => LogLevel::FAIL,

            // Orchestrator
            LogCode::SCHED => LogLevel::INFO,
            LogCode::STARTFAIL => LogLevel::FAIL,

            // Manager
            LogCode::INVALIDCAP => LogLevel::FAIL,
            LogCode::QUNAVAILABLE => LogLevel::FAIL,
            LogCode::QUEUED => LogLevel::INFO,
            LogCode::NALLOC => LogLevel::INFO,
            LogCode::PENDING => LogLevel::INFO,
            LogCode::NALIVE => LogLevel::INFO,

            LogCode::CLEFT => LogLevel::WARN,

            LogCode::QTIMEOUT => LogLevel::FAIL,
            LogCode::OTIMEOUT => LogLevel::FAIL,
            LogCode::NTIMEOUT => LogLevel::FAIL,
        }
    }
}

impl fmt::Display for LogCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}