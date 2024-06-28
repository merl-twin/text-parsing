use crate::{
    Error,
};

#[derive(Debug,Clone,Copy)]
pub struct Pos {
    pub offset: usize,
    pub length: usize,
}

pub trait Localize: Sized {
    fn localize(self, chars: Pos, bytes: Pos) -> Local<Self> {
        Local { chars, bytes, data: self }
    }
}
impl<T: Sized> Localize for T {}

#[derive(Debug,Clone,Copy)]
pub struct Local<E> {
    chars: Pos,
    bytes: Pos,
    data: E,
}
impl<E> Local<E> {
    pub fn into_inner(self) -> (Local<()>,E) {
        (Local {
            chars: self.chars,
            bytes: self.bytes,
            data: (),
        }, self.data)
    }
    pub fn data(&self) -> &E {
        &self.data
    }

    pub fn local<T>(&self, data: T) -> Local<T>
    {
        Local {
            chars: self.chars,
            bytes: self.bytes,
            data,
        }
    }
    pub fn map<F,T>(self, mut mapper: F) -> Local<T>
    where F: FnMut(E) -> T
    {
        Local {
            chars: self.chars,
            bytes: self.bytes,
            data: mapper(self.data),
        }
    }
    pub fn with_inner<T>(self, inner: T) -> Local<T> {
        Local {
            chars: self.chars,
            bytes: self.bytes,
            data: inner,
        }
    }

    pub fn from_segment<T>(begin: Local<E>, end: Local<T>) -> Result<Local<E>,Error> {
        if (begin.chars.offset <= end.chars.offset) && (begin.bytes.offset <= end.bytes.offset) {
            Ok(Local {
                chars: Pos {
                    offset: begin.chars.offset,
                    length: end.chars.length + end.chars.offset - begin.chars.offset,
                },
                bytes: Pos {
                    offset: begin.bytes.offset,
                    length: end.bytes.length + end.bytes.offset - begin.bytes.offset,
                },
                data: begin.data,
            })
        } else {
            Err(Error::EndBeforeBegin)
        }
    }
}
