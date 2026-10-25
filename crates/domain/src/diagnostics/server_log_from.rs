use crate::diagnostics::ServerLogCursor;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerLogFrom {
    Newest,
    Before(ServerLogCursor),
    After(ServerLogCursor),
}
