//! Router identification and port-forwarding guide engines, ported from MSC 1's
//! five `RouterPortForward*` files (2,077 lines total, zero MSC 1 test coverage
//! for any of them — `rolling-plan.md` P1.10-P1.14). `matcher` is the first;
//! the fallback decision tree, guide composer, troubleshooting engine, and
//! runtime resolver land in their own modules as their own steps port them.

pub mod composer;
pub mod fallback_tree;
pub mod matcher;
