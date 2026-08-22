//! WebSocket channels. `console` (P2.15) is the one channel P0.24 found
//! MSC 1 actually has; `operations` (P2.16) is greenfield MSC 2 design
//! pushing `OperationDTO` updates, per `websocket-v1.json`.

pub mod console;
pub mod notifications;
pub mod operations;
