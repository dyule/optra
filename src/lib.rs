
#![feature(custom_derive, plugin, linked_list_extras)]
#![plugin(serde_macros)]
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate log;

mod operations;
pub mod engine;

pub type Offset = i64;
pub type Position = u64;

pub struct OTError {
    kind: ErrorKind
}

pub enum ErrorKind {
    NoSuchState
}

#[derive(PartialEq, Debug)]
pub enum OverlapResult {
    Precedes,
    Follows,
    Encloses,
    OverlapFront,
    OverlapBack,
    EnclosedBy
}

impl OTError {
    #[inline]
    pub fn new(kind: ErrorKind) -> OTError {
        OTError {
            kind: kind
        }
    }
}
