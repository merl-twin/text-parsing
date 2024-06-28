
use opt_struct::OptVec;

use crate::Breaker;

#[derive(Debug)]
pub struct Tag {
    pub name: TagName,
    pub breaker: Breaker,
    pub closing: Closing,
    pub attributes: OptVec<(String, Option<String>)>,
}



impl Tag {
   /* pub fn from_slice(s: &str) -> Tag {
        
}*/
    pub fn new(tag: TagName, clo: Closing, attrs: OptVec<(String, Option<String>)>) -> Tag {
        Tag {
            breaker: tag.breaker(),
            name: tag,
            closing: clo,
            attributes: attrs,
        }
    }
}

#[derive(Debug,Clone,Copy)]
pub enum Closing {
    Void,
    Open,
    Close,
}

#[derive(Debug,Clone,Copy,Eq,PartialEq)]
pub enum SpecTag {
    Slash,
    Excl,
    Quest,
}

#[derive(Debug,Clone,Eq,PartialEq)]
pub enum TagName {
    X(SpecTag),
    
    Html,
    Head,
    Title,
    Body,
    Td,
    Br,
    Hr,
    H1,
    P,
    A,
    Img,
    Sup,
    Sub,
    I,
    B,
    Wbr,
    Other(String),
}
impl TagName {
    pub fn from(s: String) -> TagName {
        match &s as &str {
            "html" => TagName::Html,
            "head" => TagName::Head,
            "title" => TagName::Title,
            "body" => TagName::Body,
            "td" => TagName::Td,
            "br" => TagName::Br,
            "hr" => TagName::Hr,
            "h1" => TagName::H1,
            "p" => TagName::P,
            "a" => TagName::A,
            "img" => TagName::Img,
            "sup" => TagName::Sup,
            "sub" => TagName::Sub,
            "i" => TagName::I,
            "b" => TagName::B,
            "wbr" => TagName::Wbr,
            _ => TagName::Other(s),
        }
    }
    pub fn x_from(s: SpecTag) -> TagName {
        TagName::X(s)
    }
    fn breaker(&self) -> Breaker {
        match self {
            TagName::Html |
            TagName::Title |
            TagName::Head |
            TagName::Body |
            TagName::H1 => Breaker::Section,
            TagName::Hr |
            TagName::P => Breaker::Paragraph,
            TagName::Br |
            TagName::Td => Breaker::Sentence,
            TagName::A |
            TagName::Img |
            TagName::Sup |
            TagName::Sub => Breaker::Word,
            TagName::I |
            TagName::B |
            TagName::Wbr => Breaker::None,

            TagName::X(_) => Breaker::Word,
            TagName::Other(_name) => Breaker::Word,
        }
    }
}
