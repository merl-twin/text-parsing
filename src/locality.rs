use crate::{
    Error,
};

#[derive(Debug,Clone,Copy,Eq,PartialEq)]
pub struct Snip {
    pub offset: usize,
    pub length: usize,
}

pub trait Localize: Sized {
    fn localize(self, chars: Snip, bytes: Snip) -> Local<Self> {
        Local { chars, bytes, data: self }
    }
}
impl<T: Sized> Localize for T {}

#[derive(Debug,Clone,Copy,Eq,PartialEq)]
pub struct Local<E> {
    chars: Snip,
    bytes: Snip,
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
    pub fn chars(&self) -> Snip {
        self.chars
    }
    pub fn bytes(&self) -> Snip {
        self.bytes
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
    pub fn with_shift(mut self, char_offset: usize, byte_offset: usize) -> Local<E> {
        self.chars.offset += char_offset;
        self.bytes.offset += byte_offset;
        self
    }

    pub fn from_segment<T>(begin: Local<E>, end: Local<T>) -> Result<Local<E>,Error> {
        if (begin.chars.offset <= end.chars.offset) && (begin.bytes.offset <= end.bytes.offset) {
            Ok(Local {
                chars: Snip {
                    offset: begin.chars.offset,
                    length: end.chars.length + end.chars.offset - begin.chars.offset,
                },
                bytes: Snip {
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
