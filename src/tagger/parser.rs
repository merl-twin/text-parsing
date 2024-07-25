use super::{
    tags::{
        Tag, TagName,
    },
    state::{
        TaggerState,
    }
};
use crate::{
    ParserResult,
    Source,
    Parser, Runtime,
};

/*

  Algorithm: https://dev.w3.org/html5/spec-LC/parsing.html

*/


#[derive(Debug,Clone)]
pub struct Builder {
    properties: TaggerProperties,
}
impl Builder {
    pub fn new() -> Builder {
        Builder{
            properties: TaggerProperties::default(),
        }
    }
    pub fn with_all_attributes(mut self) -> Builder {
        self.properties.attributes = AttributeProperties::All;
        self
    }
    pub fn with_attribute<S: ToString>(mut self, name: TagName, attr: S) -> Builder {
        match &mut self.properties.attributes {
            AttributeProperties::None |
            AttributeProperties::All => self.properties.attributes = AttributeProperties::Custom(vec![(name,attr.to_string())]),
            AttributeProperties::Custom(v) => v.push((name,attr.to_string())),
        };
        self
    }
    pub fn skip_eof_in_tag(mut self) -> Builder {
        self.properties.eof_in_tag = Unknown::Skip;
        self
    }
    pub fn create(self) -> TagParser {
        TagParser(Runtime::new(self.properties))
    }
}

#[derive(Debug,Clone,Copy)]
pub(in super) enum Unknown {
    Error,
    Skip,
    //Text,
}

#[derive(Debug,Clone)]
pub(in super) enum AttributeProperties {
    None,
    Custom(Vec<(TagName,String)>),
    All,
}

#[derive(Debug,Clone)]
pub(in super) struct TaggerProperties {
    pub eof_in_tag: Unknown,

    pub attributes: AttributeProperties,
}
impl Default for TaggerProperties {
    fn default() -> TaggerProperties {
        TaggerProperties {
            eof_in_tag: Unknown::Error,
            attributes: AttributeProperties::None,
        }
    }
}

pub struct TagParser(Runtime<TaggerState,Tag,TaggerProperties>);

impl Parser for TagParser {
    type Data = Tag;
    
    fn next_event<S: Source>(&mut self, src: &mut S) -> ParserResult<Tag> {
        self.0.next_event(src)
    }
}

/*impl PipeParser for TagParser {
    fn next_char<S: Source>(&mut self, src: &mut S) -> SourceResult {
        Ok(match self.next_event(src)? {
            Some(local_pe) => {
                let (local,pe) = local_pe.into_inner();
                Some(local.local(match pe {
                    ParserEvent::Char(c) => SourceEvent::Char(c),
                    ParserEvent::Breaker(b) => SourceEvent::Breaker(b),
                    ParserEvent::Parsed(tag) => SourceEvent::Breaker(tag.breaker),
                }))
            },
            None => None,
        })
    }
}*/



#[cfg(test)]
mod tests {
    use crate::*;
    use super::*;

    use crate::tagger::tags::*;
    use opt_struct::OptVec;
    
    #[test]
    fn basic() {
        let mut src = "<h1>Hello, world!</h1>Привет, мир!".into_source();
        let mut parser = Builder::new().create();

        let mut res_iter = [
            ParserEvent::Parsed(Tag {
                name: TagName::H1, closing: Closing::Open, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 0, length: 1 },Snip { offset: 0, length: 1 }),
                end: ().localize(Snip { offset: 3, length: 1 },Snip { offset: 3, length: 1 }),
            }).localize(Snip { offset: 0, length: 4 },Snip { offset: 0, length: 4 }),
            ParserEvent::Char('H').localize(Snip { offset: 4, length: 1 },Snip { offset: 4, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 5, length: 1 },Snip { offset: 5, length: 1 }),
            ParserEvent::Char('l').localize(Snip { offset: 6, length: 1 },Snip { offset: 6, length: 1 }),
            ParserEvent::Char('l').localize(Snip { offset: 7, length: 1 },Snip { offset: 7, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 8, length: 1 },Snip { offset: 8, length: 1 }),
            ParserEvent::Char(',').localize(Snip { offset: 9, length: 1 },Snip { offset: 9, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 10, length: 1 },Snip { offset: 10, length: 1 }),
            ParserEvent::Char('w').localize(Snip { offset: 11, length: 1 },Snip { offset: 11, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 12, length: 1 },Snip { offset: 12, length: 1 }),
            ParserEvent::Char('r').localize(Snip { offset: 13, length: 1 },Snip { offset: 13, length: 1 }),
            ParserEvent::Char('l').localize(Snip { offset: 14, length: 1 },Snip { offset: 14, length: 1 }),
            ParserEvent::Char('d').localize(Snip { offset: 15, length: 1 },Snip { offset: 15, length: 1 }),
            ParserEvent::Char('!').localize(Snip { offset: 16, length: 1 },Snip { offset: 16, length: 1 }),
            ParserEvent::Parsed(Tag {
                name: TagName::H1, closing: Closing::Close, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 17, length: 1 },Snip { offset: 17, length: 1 }),
                end: ().localize(Snip { offset: 21, length: 1 },Snip { offset: 21, length: 1 }),
            }).localize(Snip { offset: 17, length: 5 },Snip { offset: 17, length: 5 }),
            ParserEvent::Char('П').localize(Snip { offset: 22, length: 1 },Snip { offset: 22, length: 2 }),
            ParserEvent::Char('р').localize(Snip { offset: 23, length: 1 },Snip { offset: 24, length: 2 }),
            ParserEvent::Char('и').localize(Snip { offset: 24, length: 1 },Snip { offset: 26, length: 2 }),
            ParserEvent::Char('в').localize(Snip { offset: 25, length: 1 },Snip { offset: 28, length: 2 }),
            ParserEvent::Char('е').localize(Snip { offset: 26, length: 1 },Snip { offset: 30, length: 2 }),
            ParserEvent::Char('т').localize(Snip { offset: 27, length: 1 },Snip { offset: 32, length: 2 }),
            ParserEvent::Char(',').localize(Snip { offset: 28, length: 1 },Snip { offset: 34, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 29, length: 1 },Snip { offset: 35, length: 1 }),
            ParserEvent::Char('м').localize(Snip { offset: 30, length: 1 },Snip { offset: 36, length: 2 }),
            ParserEvent::Char('и').localize(Snip { offset: 31, length: 1 },Snip { offset: 38, length: 2 }),
            ParserEvent::Char('р').localize(Snip { offset: 32, length: 1 },Snip { offset: 40, length: 2 }),
            ParserEvent::Char('!').localize(Snip { offset: 33, length: 1 },Snip { offset: 42, length: 1 }),
        ].into_iter();

        while let Some(local_event) = parser.next_event(&mut src).unwrap() {
            //let (local,event) = local_event.into_inner();
            //println!("ParserEvent::{:?}.localize({:?},{:?}),",event,local.chars(),local.bytes());
            match res_iter.next() {
                Some(ev) => {
                    println!("Parser: {:?}",local_event);
                    println!("Result: {:?}",ev);
                    assert_eq!(local_event,ev);
                },
                None => {
                    panic!("parser has more events then test result");
                },
            }
        }
    }

    #[test]
    fn basic_void() {
        let mut src = "<h1>Hello, world!</h1>Привет, <tag />мир!".into_source();
        let mut parser = Builder::new().create();

        let mut res_iter = [
            ParserEvent::Parsed(Tag {
                name: TagName::H1, closing: Closing::Open, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 0, length: 1 },Snip { offset: 0, length: 1 }),
                end: ().localize(Snip { offset: 3, length: 1 },Snip { offset: 3, length: 1 }),
            }).localize(Snip { offset: 0, length: 4 },Snip { offset: 0, length: 4 }),
            ParserEvent::Char('H').localize(Snip { offset: 4, length: 1 },Snip { offset: 4, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 5, length: 1 },Snip { offset: 5, length: 1 }),
            ParserEvent::Char('l').localize(Snip { offset: 6, length: 1 },Snip { offset: 6, length: 1 }),
            ParserEvent::Char('l').localize(Snip { offset: 7, length: 1 },Snip { offset: 7, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 8, length: 1 },Snip { offset: 8, length: 1 }),
            ParserEvent::Char(',').localize(Snip { offset: 9, length: 1 },Snip { offset: 9, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 10, length: 1 },Snip { offset: 10, length: 1 }),
            ParserEvent::Char('w').localize(Snip { offset: 11, length: 1 },Snip { offset: 11, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 12, length: 1 },Snip { offset: 12, length: 1 }),
            ParserEvent::Char('r').localize(Snip { offset: 13, length: 1 },Snip { offset: 13, length: 1 }),
            ParserEvent::Char('l').localize(Snip { offset: 14, length: 1 },Snip { offset: 14, length: 1 }),
            ParserEvent::Char('d').localize(Snip { offset: 15, length: 1 },Snip { offset: 15, length: 1 }),
            ParserEvent::Char('!').localize(Snip { offset: 16, length: 1 },Snip { offset: 16, length: 1 }),
            ParserEvent::Parsed(Tag {
                name: TagName::H1, closing: Closing::Close, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 17, length: 1 },Snip { offset: 17, length: 1 }),
                end: ().localize(Snip { offset: 21, length: 1 },Snip { offset: 21, length: 1 }),
            }).localize(Snip { offset: 17, length: 5 },Snip { offset: 17, length: 5 }),
            ParserEvent::Char('П').localize(Snip { offset: 22, length: 1 },Snip { offset: 22, length: 2 }),
            ParserEvent::Char('р').localize(Snip { offset: 23, length: 1 },Snip { offset: 24, length: 2 }),
            ParserEvent::Char('и').localize(Snip { offset: 24, length: 1 },Snip { offset: 26, length: 2 }),
            ParserEvent::Char('в').localize(Snip { offset: 25, length: 1 },Snip { offset: 28, length: 2 }),
            ParserEvent::Char('е').localize(Snip { offset: 26, length: 1 },Snip { offset: 30, length: 2 }),
            ParserEvent::Char('т').localize(Snip { offset: 27, length: 1 },Snip { offset: 32, length: 2 }),
            ParserEvent::Char(',').localize(Snip { offset: 28, length: 1 },Snip { offset: 34, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 29, length: 1 },Snip { offset: 35, length: 1 }),
            ParserEvent::Parsed(Tag {
                name: TagName::Other("tag".to_string()), closing: Closing::Void, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 30, length: 1 },Snip { offset: 36, length: 1 }),
                end: ().localize(Snip { offset: 36, length: 1 },Snip { offset: 42, length: 1 }),
            }).localize(Snip { offset: 30, length: 7 },Snip { offset: 36, length: 7 }),
            ParserEvent::Char('м').localize(Snip { offset: 37, length: 1 },Snip { offset: 43, length: 2 }),
            ParserEvent::Char('и').localize(Snip { offset: 38, length: 1 },Snip { offset: 45, length: 2 }),
            ParserEvent::Char('р').localize(Snip { offset: 39, length: 1 },Snip { offset: 47, length: 2 }),
            ParserEvent::Char('!').localize(Snip { offset: 40, length: 1 },Snip { offset: 49, length: 1 }),
        ].into_iter();

        while let Some(local_event) = parser.next_event(&mut src).unwrap() {
            //let (local,event) = local_event.into_inner();
            //println!("ParserEvent::{:?}.localize({:?},{:?}),",event,local.chars(),local.bytes());
            match res_iter.next() {
                Some(ev) => {
                    println!("Parser: {:?}",local_event);
                    println!("Result: {:?}",ev);
                    assert_eq!(local_event,ev);
                },
                None => {
                    panic!("parser has more events then test result");
                },
            }
        }
    }

    #[test]
    fn basic_void_2() {
        let mut src = "<h1>Hello, world!</h1>Привет, <tags/>мир!".into_source();
        let mut parser = Builder::new().create();

        let mut res_iter = [
            ParserEvent::Parsed(Tag {
                name: TagName::H1, closing: Closing::Open, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 0, length: 1 },Snip { offset: 0, length: 1 }),
                end: ().localize(Snip { offset: 3, length: 1 },Snip { offset: 3, length: 1 }),
            }).localize(Snip { offset: 0, length: 4 },Snip { offset: 0, length: 4 }),
            ParserEvent::Char('H').localize(Snip { offset: 4, length: 1 },Snip { offset: 4, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 5, length: 1 },Snip { offset: 5, length: 1 }),
            ParserEvent::Char('l').localize(Snip { offset: 6, length: 1 },Snip { offset: 6, length: 1 }),
            ParserEvent::Char('l').localize(Snip { offset: 7, length: 1 },Snip { offset: 7, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 8, length: 1 },Snip { offset: 8, length: 1 }),
            ParserEvent::Char(',').localize(Snip { offset: 9, length: 1 },Snip { offset: 9, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 10, length: 1 },Snip { offset: 10, length: 1 }),
            ParserEvent::Char('w').localize(Snip { offset: 11, length: 1 },Snip { offset: 11, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 12, length: 1 },Snip { offset: 12, length: 1 }),
            ParserEvent::Char('r').localize(Snip { offset: 13, length: 1 },Snip { offset: 13, length: 1 }),
            ParserEvent::Char('l').localize(Snip { offset: 14, length: 1 },Snip { offset: 14, length: 1 }),
            ParserEvent::Char('d').localize(Snip { offset: 15, length: 1 },Snip { offset: 15, length: 1 }),
            ParserEvent::Char('!').localize(Snip { offset: 16, length: 1 },Snip { offset: 16, length: 1 }),
            ParserEvent::Parsed(Tag {
                name: TagName::H1, closing: Closing::Close, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 17, length: 1 },Snip { offset: 17, length: 1 }),
                end: ().localize(Snip { offset: 21, length: 1 },Snip { offset: 21, length: 1 }),
            }).localize(Snip { offset: 17, length: 5 },Snip { offset: 17, length: 5 }),
            ParserEvent::Char('П').localize(Snip { offset: 22, length: 1 },Snip { offset: 22, length: 2 }),
            ParserEvent::Char('р').localize(Snip { offset: 23, length: 1 },Snip { offset: 24, length: 2 }),
            ParserEvent::Char('и').localize(Snip { offset: 24, length: 1 },Snip { offset: 26, length: 2 }),
            ParserEvent::Char('в').localize(Snip { offset: 25, length: 1 },Snip { offset: 28, length: 2 }),
            ParserEvent::Char('е').localize(Snip { offset: 26, length: 1 },Snip { offset: 30, length: 2 }),
            ParserEvent::Char('т').localize(Snip { offset: 27, length: 1 },Snip { offset: 32, length: 2 }),
            ParserEvent::Char(',').localize(Snip { offset: 28, length: 1 },Snip { offset: 34, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 29, length: 1 },Snip { offset: 35, length: 1 }),
            ParserEvent::Parsed(Tag {
                name: TagName::Other("tags".to_string()), closing: Closing::Void, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 30, length: 1 },Snip { offset: 36, length: 1 }),
                end: ().localize(Snip { offset: 36, length: 1 },Snip { offset: 42, length: 1 }),
            }).localize(Snip { offset: 30, length: 7 },Snip { offset: 36, length: 7 }),
            ParserEvent::Char('м').localize(Snip { offset: 37, length: 1 },Snip { offset: 43, length: 2 }),
            ParserEvent::Char('и').localize(Snip { offset: 38, length: 1 },Snip { offset: 45, length: 2 }),
            ParserEvent::Char('р').localize(Snip { offset: 39, length: 1 },Snip { offset: 47, length: 2 }),
            ParserEvent::Char('!').localize(Snip { offset: 40, length: 1 },Snip { offset: 49, length: 1 }),
        ].into_iter();

        while let Some(local_event) = parser.next_event(&mut src).unwrap() {
            //let (local,event) = local_event.into_inner();
            //println!("ParserEvent::{:?}.localize({:?},{:?}),",event,local.chars(),local.bytes());
            match res_iter.next() {
                Some(ev) => {
                    println!("Parser: {:?}",local_event);
                    println!("Result: {:?}",ev);
                    assert_eq!(local_event,ev);
                },
                None => {
                    panic!("parser has more events then test result");
                },
            }
        }
    }

    
    #[test]
    fn a_img() {        
        let mut src = "
<p>In the common case, <a href=\"apis-in-html-documents.html#dynamic-markup-insertion\" title=\"dynamic markup
  insertion\">, e.g. using the <code title=\"dom-document-write\"><a href=\"apis-in-html-documents.html#dom-document-write\">document.write()</a></code> API.</p>
  <p><img alt=\"\" height=\"554\" src=\"https://dev.w3.org/html5/spec/images/parsing-model-overview.png\" width=\"427\"></p>
  <p id=\"nestedParsing\">There is only one set of states for the
  tokenizer stage and the tree construction stage...</p>".into_source();
        let mut parser = Builder::new()
            .with_attribute(TagName::A,"href")
            .with_attribute(TagName::Img,"alt")
            .create();

        let mut res_iter = [
            ParserEvent::Char('\n').localize(Snip { offset: 0, length: 1 },Snip { offset: 0, length: 1 }),
            ParserEvent::Parsed(Tag {
                name: TagName::P, closing: Closing::Open, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 1, length: 1 },Snip { offset: 1, length: 1 }),
                end: ().localize(Snip { offset: 3, length: 1 },Snip { offset: 3, length: 1 }),
            }).localize(Snip { offset: 1, length: 3 },Snip { offset: 1, length: 3 }),
            ParserEvent::Char('I').localize(Snip { offset: 4, length: 1 },Snip { offset: 4, length: 1 }),
            ParserEvent::Char('n').localize(Snip { offset: 5, length: 1 },Snip { offset: 5, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 6, length: 1 },Snip { offset: 6, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 7, length: 1 },Snip { offset: 7, length: 1 }),
            ParserEvent::Char('h').localize(Snip { offset: 8, length: 1 },Snip { offset: 8, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 9, length: 1 },Snip { offset: 9, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 10, length: 1 },Snip { offset: 10, length: 1 }),
            ParserEvent::Char('c').localize(Snip { offset: 11, length: 1 },Snip { offset: 11, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 12, length: 1 },Snip { offset: 12, length: 1 }),
            ParserEvent::Char('m').localize(Snip { offset: 13, length: 1 },Snip { offset: 13, length: 1 }),
            ParserEvent::Char('m').localize(Snip { offset: 14, length: 1 },Snip { offset: 14, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 15, length: 1 },Snip { offset: 15, length: 1 }),
            ParserEvent::Char('n').localize(Snip { offset: 16, length: 1 },Snip { offset: 16, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 17, length: 1 },Snip { offset: 17, length: 1 }),
            ParserEvent::Char('c').localize(Snip { offset: 18, length: 1 },Snip { offset: 18, length: 1 }),
            ParserEvent::Char('a').localize(Snip { offset: 19, length: 1 },Snip { offset: 19, length: 1 }),
            ParserEvent::Char('s').localize(Snip { offset: 20, length: 1 },Snip { offset: 20, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 21, length: 1 },Snip { offset: 21, length: 1 }),
            ParserEvent::Char(',').localize(Snip { offset: 22, length: 1 },Snip { offset: 22, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 23, length: 1 },Snip { offset: 23, length: 1 }),
            ParserEvent::Parsed(Tag {
                name: TagName::A, closing: Closing::Open,
                attributes: OptVec::One(("href".to_string(), Some(vec![
                    SourceEvent::Char('a').localize(Snip { offset: 33, length: 1 },Snip { offset: 33, length: 1 }),
                    SourceEvent::Char('p').localize(Snip { offset: 34, length: 1 },Snip { offset: 34, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 35, length: 1 },Snip { offset: 35, length: 1 }),
                    SourceEvent::Char('s').localize(Snip { offset: 36, length: 1 },Snip { offset: 36, length: 1 }),
                    SourceEvent::Char('-').localize(Snip { offset: 37, length: 1 },Snip { offset: 37, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 38, length: 1 },Snip { offset: 38, length: 1 }),
                    SourceEvent::Char('n').localize(Snip { offset: 39, length: 1 },Snip { offset: 39, length: 1 }),
                    SourceEvent::Char('-').localize(Snip { offset: 40, length: 1 },Snip { offset: 40, length: 1 }),
                    SourceEvent::Char('h').localize(Snip { offset: 41, length: 1 },Snip { offset: 41, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 42, length: 1 },Snip { offset: 42, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 43, length: 1 },Snip { offset: 43, length: 1 }),
                    SourceEvent::Char('l').localize(Snip { offset: 44, length: 1 },Snip { offset: 44, length: 1 }),
                    SourceEvent::Char('-').localize(Snip { offset: 45, length: 1 },Snip { offset: 45, length: 1 }),
                    SourceEvent::Char('d').localize(Snip { offset: 46, length: 1 },Snip { offset: 46, length: 1 }),
                    SourceEvent::Char('o').localize(Snip { offset: 47, length: 1 },Snip { offset: 47, length: 1 }),
                    SourceEvent::Char('c').localize(Snip { offset: 48, length: 1 },Snip { offset: 48, length: 1 }),
                    SourceEvent::Char('u').localize(Snip { offset: 49, length: 1 },Snip { offset: 49, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 50, length: 1 },Snip { offset: 50, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 51, length: 1 },Snip { offset: 51, length: 1 }),
                    SourceEvent::Char('n').localize(Snip { offset: 52, length: 1 },Snip { offset: 52, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 53, length: 1 },Snip { offset: 53, length: 1 }),
                    SourceEvent::Char('s').localize(Snip { offset: 54, length: 1 },Snip { offset: 54, length: 1 }),
                    SourceEvent::Char('.').localize(Snip { offset: 55, length: 1 },Snip { offset: 55, length: 1 }),
                    SourceEvent::Char('h').localize(Snip { offset: 56, length: 1 },Snip { offset: 56, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 57, length: 1 },Snip { offset: 57, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 58, length: 1 },Snip { offset: 58, length: 1 }),
                    SourceEvent::Char('l').localize(Snip { offset: 59, length: 1 },Snip { offset: 59, length: 1 }),
                    SourceEvent::Char('#').localize(Snip { offset: 60, length: 1 },Snip { offset: 60, length: 1 }),
                    SourceEvent::Char('d').localize(Snip { offset: 61, length: 1 },Snip { offset: 61, length: 1 }),
                    SourceEvent::Char('y').localize(Snip { offset: 62, length: 1 },Snip { offset: 62, length: 1 }),
                    SourceEvent::Char('n').localize(Snip { offset: 63, length: 1 },Snip { offset: 63, length: 1 }),
                    SourceEvent::Char('a').localize(Snip { offset: 64, length: 1 },Snip { offset: 64, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 65, length: 1 },Snip { offset: 65, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 66, length: 1 },Snip { offset: 66, length: 1 }),
                    SourceEvent::Char('c').localize(Snip { offset: 67, length: 1 },Snip { offset: 67, length: 1 }),
                    SourceEvent::Char('-').localize(Snip { offset: 68, length: 1 },Snip { offset: 68, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 69, length: 1 },Snip { offset: 69, length: 1 }),
                    SourceEvent::Char('a').localize(Snip { offset: 70, length: 1 },Snip { offset: 70, length: 1 }),
                    SourceEvent::Char('r').localize(Snip { offset: 71, length: 1 },Snip { offset: 71, length: 1 }),
                    SourceEvent::Char('k').localize(Snip { offset: 72, length: 1 },Snip { offset: 72, length: 1 }),
                    SourceEvent::Char('u').localize(Snip { offset: 73, length: 1 },Snip { offset: 73, length: 1 }),
                    SourceEvent::Char('p').localize(Snip { offset: 74, length: 1 },Snip { offset: 74, length: 1 }),
                    SourceEvent::Char('-').localize(Snip { offset: 75, length: 1 },Snip { offset: 75, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 76, length: 1 },Snip { offset: 76, length: 1 }),
                    SourceEvent::Char('n').localize(Snip { offset: 77, length: 1 },Snip { offset: 77, length: 1 }),
                    SourceEvent::Char('s').localize(Snip { offset: 78, length: 1 },Snip { offset: 78, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 79, length: 1 },Snip { offset: 79, length: 1 }),
                    SourceEvent::Char('r').localize(Snip { offset: 80, length: 1 },Snip { offset: 80, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 81, length: 1 },Snip { offset: 81, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 82, length: 1 },Snip { offset: 82, length: 1 }),
                    SourceEvent::Char('o').localize(Snip { offset: 83, length: 1 },Snip { offset: 83, length: 1 }),
                    SourceEvent::Char('n').localize(Snip { offset: 84, length: 1 },Snip { offset: 84, length: 1 }),
                ]))),
                begin: ().localize(Snip { offset: 24, length: 1 },Snip { offset: 24, length: 1 }),
                end: ().localize(Snip { offset: 121, length: 1 },Snip { offset: 121, length: 1 }),
            }).localize(Snip { offset: 24, length: 98 },Snip { offset: 24, length: 98 }),                
            ParserEvent::Char(',').localize(Snip { offset: 122, length: 1 },Snip { offset: 122, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 123, length: 1 },Snip { offset: 123, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 124, length: 1 },Snip { offset: 124, length: 1 }),
            ParserEvent::Char('.').localize(Snip { offset: 125, length: 1 },Snip { offset: 125, length: 1 }),
            ParserEvent::Char('g').localize(Snip { offset: 126, length: 1 },Snip { offset: 126, length: 1 }),
            ParserEvent::Char('.').localize(Snip { offset: 127, length: 1 },Snip { offset: 127, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 128, length: 1 },Snip { offset: 128, length: 1 }),
            ParserEvent::Char('u').localize(Snip { offset: 129, length: 1 },Snip { offset: 129, length: 1 }),
            ParserEvent::Char('s').localize(Snip { offset: 130, length: 1 },Snip { offset: 130, length: 1 }),
            ParserEvent::Char('i').localize(Snip { offset: 131, length: 1 },Snip { offset: 131, length: 1 }),
            ParserEvent::Char('n').localize(Snip { offset: 132, length: 1 },Snip { offset: 132, length: 1 }),
            ParserEvent::Char('g').localize(Snip { offset: 133, length: 1 },Snip { offset: 133, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 134, length: 1 },Snip { offset: 134, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 135, length: 1 },Snip { offset: 135, length: 1 }),
            ParserEvent::Char('h').localize(Snip { offset: 136, length: 1 },Snip { offset: 136, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 137, length: 1 },Snip { offset: 137, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 138, length: 1 },Snip { offset: 138, length: 1 }),
            ParserEvent::Parsed(Tag {
                name: TagName::Code, closing: Closing::Open, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 139, length: 1 },Snip { offset: 139, length: 1 }),
                end: ().localize(Snip { offset: 171, length: 1 },Snip { offset: 171, length: 1 }),
            }).localize(Snip { offset: 139, length: 33 },Snip { offset: 139, length: 33 }),
            ParserEvent::Parsed(Tag {
                name: TagName::A, closing: Closing::Open,
                attributes: OptVec::One(("href".to_string(), Some(vec![
                    SourceEvent::Char('a').localize(Snip { offset: 181, length: 1 },Snip { offset: 181, length: 1 }),
                    SourceEvent::Char('p').localize(Snip { offset: 182, length: 1 },Snip { offset: 182, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 183, length: 1 },Snip { offset: 183, length: 1 }),
                    SourceEvent::Char('s').localize(Snip { offset: 184, length: 1 },Snip { offset: 184, length: 1 }),
                    SourceEvent::Char('-').localize(Snip { offset: 185, length: 1 },Snip { offset: 185, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 186, length: 1 },Snip { offset: 186, length: 1 }),
                    SourceEvent::Char('n').localize(Snip { offset: 187, length: 1 },Snip { offset: 187, length: 1 }),
                    SourceEvent::Char('-').localize(Snip { offset: 188, length: 1 },Snip { offset: 188, length: 1 }),
                    SourceEvent::Char('h').localize(Snip { offset: 189, length: 1 },Snip { offset: 189, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 190, length: 1 },Snip { offset: 190, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 191, length: 1 },Snip { offset: 191, length: 1 }),
                    SourceEvent::Char('l').localize(Snip { offset: 192, length: 1 },Snip { offset: 192, length: 1 }),
                    SourceEvent::Char('-').localize(Snip { offset: 193, length: 1 },Snip { offset: 193, length: 1 }),
                    SourceEvent::Char('d').localize(Snip { offset: 194, length: 1 },Snip { offset: 194, length: 1 }),
                    SourceEvent::Char('o').localize(Snip { offset: 195, length: 1 },Snip { offset: 195, length: 1 }),
                    SourceEvent::Char('c').localize(Snip { offset: 196, length: 1 },Snip { offset: 196, length: 1 }),
                    SourceEvent::Char('u').localize(Snip { offset: 197, length: 1 },Snip { offset: 197, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 198, length: 1 },Snip { offset: 198, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 199, length: 1 },Snip { offset: 199, length: 1 }),
                    SourceEvent::Char('n').localize(Snip { offset: 200, length: 1 },Snip { offset: 200, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 201, length: 1 },Snip { offset: 201, length: 1 }),
                    SourceEvent::Char('s').localize(Snip { offset: 202, length: 1 },Snip { offset: 202, length: 1 }),
                    SourceEvent::Char('.').localize(Snip { offset: 203, length: 1 },Snip { offset: 203, length: 1 }),
                    SourceEvent::Char('h').localize(Snip { offset: 204, length: 1 },Snip { offset: 204, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 205, length: 1 },Snip { offset: 205, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 206, length: 1 },Snip { offset: 206, length: 1 }),
                    SourceEvent::Char('l').localize(Snip { offset: 207, length: 1 },Snip { offset: 207, length: 1 }),
                    SourceEvent::Char('#').localize(Snip { offset: 208, length: 1 },Snip { offset: 208, length: 1 }),
                    SourceEvent::Char('d').localize(Snip { offset: 209, length: 1 },Snip { offset: 209, length: 1 }),
                    SourceEvent::Char('o').localize(Snip { offset: 210, length: 1 },Snip { offset: 210, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 211, length: 1 },Snip { offset: 211, length: 1 }),
                    SourceEvent::Char('-').localize(Snip { offset: 212, length: 1 },Snip { offset: 212, length: 1 }),
                    SourceEvent::Char('d').localize(Snip { offset: 213, length: 1 },Snip { offset: 213, length: 1 }),
                    SourceEvent::Char('o').localize(Snip { offset: 214, length: 1 },Snip { offset: 214, length: 1 }),
                    SourceEvent::Char('c').localize(Snip { offset: 215, length: 1 },Snip { offset: 215, length: 1 }),
                    SourceEvent::Char('u').localize(Snip { offset: 216, length: 1 },Snip { offset: 216, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 217, length: 1 },Snip { offset: 217, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 218, length: 1 },Snip { offset: 218, length: 1 }),
                    SourceEvent::Char('n').localize(Snip { offset: 219, length: 1 },Snip { offset: 219, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 220, length: 1 },Snip { offset: 220, length: 1 }),
                    SourceEvent::Char('-').localize(Snip { offset: 221, length: 1 },Snip { offset: 221, length: 1 }),
                    SourceEvent::Char('w').localize(Snip { offset: 222, length: 1 },Snip { offset: 222, length: 1 }),
                    SourceEvent::Char('r').localize(Snip { offset: 223, length: 1 },Snip { offset: 223, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 224, length: 1 },Snip { offset: 224, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 225, length: 1 },Snip { offset: 225, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 226, length: 1 },Snip { offset: 226, length: 1 }),
                ]))),
                begin: ().localize(Snip { offset: 172, length: 1 },Snip { offset: 172, length: 1 }),
                end: ().localize(Snip { offset: 228, length: 1 },Snip { offset: 228, length: 1 }),
            }).localize(Snip { offset: 172, length: 57 },Snip { offset: 172, length: 57 }), 
            ParserEvent::Char('d').localize(Snip { offset: 229, length: 1 },Snip { offset: 229, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 230, length: 1 },Snip { offset: 230, length: 1 }),
            ParserEvent::Char('c').localize(Snip { offset: 231, length: 1 },Snip { offset: 231, length: 1 }),
            ParserEvent::Char('u').localize(Snip { offset: 232, length: 1 },Snip { offset: 232, length: 1 }),
            ParserEvent::Char('m').localize(Snip { offset: 233, length: 1 },Snip { offset: 233, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 234, length: 1 },Snip { offset: 234, length: 1 }),
            ParserEvent::Char('n').localize(Snip { offset: 235, length: 1 },Snip { offset: 235, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 236, length: 1 },Snip { offset: 236, length: 1 }),
            ParserEvent::Char('.').localize(Snip { offset: 237, length: 1 },Snip { offset: 237, length: 1 }),
            ParserEvent::Char('w').localize(Snip { offset: 238, length: 1 },Snip { offset: 238, length: 1 }),
            ParserEvent::Char('r').localize(Snip { offset: 239, length: 1 },Snip { offset: 239, length: 1 }),
            ParserEvent::Char('i').localize(Snip { offset: 240, length: 1 },Snip { offset: 240, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 241, length: 1 },Snip { offset: 241, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 242, length: 1 },Snip { offset: 242, length: 1 }),
            ParserEvent::Char('(').localize(Snip { offset: 243, length: 1 },Snip { offset: 243, length: 1 }),
            ParserEvent::Char(')').localize(Snip { offset: 244, length: 1 },Snip { offset: 244, length: 1 }),
            ParserEvent::Parsed(Tag {
                name: TagName::A, closing: Closing::Close, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 245, length: 1 },Snip { offset: 245, length: 1 }),
                end: ().localize(Snip { offset: 248, length: 1 },Snip { offset: 248, length: 1 }),
            }).localize(Snip { offset: 245, length: 4 },Snip { offset: 245, length: 4 }),
            ParserEvent::Parsed(Tag {
                name: TagName::Code, closing: Closing::Close, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 249, length: 1 },Snip { offset: 249, length: 1 }),
                end: ().localize(Snip { offset: 255, length: 1 },Snip { offset: 255, length: 1 }),
            }).localize(Snip { offset: 249, length: 7 },Snip { offset: 249, length: 7 }),
            ParserEvent::Char(' ').localize(Snip { offset: 256, length: 1 },Snip { offset: 256, length: 1 }),
            ParserEvent::Char('A').localize(Snip { offset: 257, length: 1 },Snip { offset: 257, length: 1 }),
            ParserEvent::Char('P').localize(Snip { offset: 258, length: 1 },Snip { offset: 258, length: 1 }),
            ParserEvent::Char('I').localize(Snip { offset: 259, length: 1 },Snip { offset: 259, length: 1 }),
            ParserEvent::Char('.').localize(Snip { offset: 260, length: 1 },Snip { offset: 260, length: 1 }),
            ParserEvent::Parsed(Tag {
                name: TagName::P, closing: Closing::Close, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 261, length: 1 },Snip { offset: 261, length: 1 }),
                end: ().localize(Snip { offset: 264, length: 1 },Snip { offset: 264, length: 1 }),
            }).localize(Snip { offset: 261, length: 4 },Snip { offset: 261, length: 4 }),
            ParserEvent::Char('\n').localize(Snip { offset: 265, length: 1 },Snip { offset: 265, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 266, length: 1 },Snip { offset: 266, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 267, length: 1 },Snip { offset: 267, length: 1 }),
            ParserEvent::Parsed(Tag {
                name: TagName::P, closing: Closing::Open, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 268, length: 1 },Snip { offset: 268, length: 1 }),
                end: ().localize(Snip { offset: 270, length: 1 },Snip { offset: 270, length: 1 }),
            }).localize(Snip { offset: 268, length: 3 },Snip { offset: 268, length: 3 }),
            ParserEvent::Parsed(Tag {
                name: TagName::Img, closing: Closing::Void,
                attributes: OptVec::One(("alt".to_string(), Some(Vec::new()))),
                begin: ().localize(Snip { offset: 271, length: 1 },Snip { offset: 271, length: 1 }),
end: ().localize(Snip { offset: 377, length: 1 },Snip { offset: 377, length: 1 }),
            }).localize(Snip { offset: 271, length: 107 },Snip { offset: 271, length: 107 }),
            ParserEvent::Parsed(Tag {
                name: TagName::P, closing: Closing::Close, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 378, length: 1 },Snip { offset: 378, length: 1 }),
                end: ().localize(Snip { offset: 381, length: 1 },Snip { offset: 381, length: 1 }),
            }).localize(Snip { offset: 378, length: 4 },Snip { offset: 378, length: 4 }),
            ParserEvent::Char('\n').localize(Snip { offset: 382, length: 1 },Snip { offset: 382, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 383, length: 1 },Snip { offset: 383, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 384, length: 1 },Snip { offset: 384, length: 1 }),
            ParserEvent::Parsed(Tag {
                name: TagName::P, closing: Closing::Open, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 385, length: 1 },Snip { offset: 385, length: 1 }),
                end: ().localize(Snip { offset: 406, length: 1 },Snip { offset: 406, length: 1 }),
            }).localize(Snip { offset: 385, length: 22 },Snip { offset: 385, length: 22 }),
            ParserEvent::Char('T').localize(Snip { offset: 407, length: 1 },Snip { offset: 407, length: 1 }),
            ParserEvent::Char('h').localize(Snip { offset: 408, length: 1 },Snip { offset: 408, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 409, length: 1 },Snip { offset: 409, length: 1 }),
            ParserEvent::Char('r').localize(Snip { offset: 410, length: 1 },Snip { offset: 410, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 411, length: 1 },Snip { offset: 411, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 412, length: 1 },Snip { offset: 412, length: 1 }),
            ParserEvent::Char('i').localize(Snip { offset: 413, length: 1 },Snip { offset: 413, length: 1 }),
            ParserEvent::Char('s').localize(Snip { offset: 414, length: 1 },Snip { offset: 414, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 415, length: 1 },Snip { offset: 415, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 416, length: 1 },Snip { offset: 416, length: 1 }),
            ParserEvent::Char('n').localize(Snip { offset: 417, length: 1 },Snip { offset: 417, length: 1 }),
            ParserEvent::Char('l').localize(Snip { offset: 418, length: 1 },Snip { offset: 418, length: 1 }),
            ParserEvent::Char('y').localize(Snip { offset: 419, length: 1 },Snip { offset: 419, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 420, length: 1 },Snip { offset: 420, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 421, length: 1 },Snip { offset: 421, length: 1 }),
            ParserEvent::Char('n').localize(Snip { offset: 422, length: 1 },Snip { offset: 422, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 423, length: 1 },Snip { offset: 423, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 424, length: 1 },Snip { offset: 424, length: 1 }),
            ParserEvent::Char('s').localize(Snip { offset: 425, length: 1 },Snip { offset: 425, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 426, length: 1 },Snip { offset: 426, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 427, length: 1 },Snip { offset: 427, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 428, length: 1 },Snip { offset: 428, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 429, length: 1 },Snip { offset: 429, length: 1 }),
            ParserEvent::Char('f').localize(Snip { offset: 430, length: 1 },Snip { offset: 430, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 431, length: 1 },Snip { offset: 431, length: 1 }),
            ParserEvent::Char('s').localize(Snip { offset: 432, length: 1 },Snip { offset: 432, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 433, length: 1 },Snip { offset: 433, length: 1 }),
            ParserEvent::Char('a').localize(Snip { offset: 434, length: 1 },Snip { offset: 434, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 435, length: 1 },Snip { offset: 435, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 436, length: 1 },Snip { offset: 436, length: 1 }),
            ParserEvent::Char('s').localize(Snip { offset: 437, length: 1 },Snip { offset: 437, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 438, length: 1 },Snip { offset: 438, length: 1 }),
            ParserEvent::Char('f').localize(Snip { offset: 439, length: 1 },Snip { offset: 439, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 440, length: 1 },Snip { offset: 440, length: 1 }),
            ParserEvent::Char('r').localize(Snip { offset: 441, length: 1 },Snip { offset: 441, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 442, length: 1 },Snip { offset: 442, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 443, length: 1 },Snip { offset: 443, length: 1 }),
            ParserEvent::Char('h').localize(Snip { offset: 444, length: 1 },Snip { offset: 444, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 445, length: 1 },Snip { offset: 445, length: 1 }),
            ParserEvent::Char('\n').localize(Snip { offset: 446, length: 1 },Snip { offset: 446, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 447, length: 1 },Snip { offset: 447, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 448, length: 1 },Snip { offset: 448, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 449, length: 1 },Snip { offset: 449, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 450, length: 1 },Snip { offset: 450, length: 1 }),
            ParserEvent::Char('k').localize(Snip { offset: 451, length: 1 },Snip { offset: 451, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 452, length: 1 },Snip { offset: 452, length: 1 }),
            ParserEvent::Char('n').localize(Snip { offset: 453, length: 1 },Snip { offset: 453, length: 1 }),
            ParserEvent::Char('i').localize(Snip { offset: 454, length: 1 },Snip { offset: 454, length: 1 }),
            ParserEvent::Char('z').localize(Snip { offset: 455, length: 1 },Snip { offset: 455, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 456, length: 1 },Snip { offset: 456, length: 1 }),
            ParserEvent::Char('r').localize(Snip { offset: 457, length: 1 },Snip { offset: 457, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 458, length: 1 },Snip { offset: 458, length: 1 }),
            ParserEvent::Char('s').localize(Snip { offset: 459, length: 1 },Snip { offset: 459, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 460, length: 1 },Snip { offset: 460, length: 1 }),
            ParserEvent::Char('a').localize(Snip { offset: 461, length: 1 },Snip { offset: 461, length: 1 }),
            ParserEvent::Char('g').localize(Snip { offset: 462, length: 1 },Snip { offset: 462, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 463, length: 1 },Snip { offset: 463, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 464, length: 1 },Snip { offset: 464, length: 1 }),
            ParserEvent::Char('a').localize(Snip { offset: 465, length: 1 },Snip { offset: 465, length: 1 }),
            ParserEvent::Char('n').localize(Snip { offset: 466, length: 1 },Snip { offset: 466, length: 1 }),
            ParserEvent::Char('d').localize(Snip { offset: 467, length: 1 },Snip { offset: 467, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 468, length: 1 },Snip { offset: 468, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 469, length: 1 },Snip { offset: 469, length: 1 }),
            ParserEvent::Char('h').localize(Snip { offset: 470, length: 1 },Snip { offset: 470, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 471, length: 1 },Snip { offset: 471, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 472, length: 1 },Snip { offset: 472, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 473, length: 1 },Snip { offset: 473, length: 1 }),
            ParserEvent::Char('r').localize(Snip { offset: 474, length: 1 },Snip { offset: 474, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 475, length: 1 },Snip { offset: 475, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 476, length: 1 },Snip { offset: 476, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 477, length: 1 },Snip { offset: 477, length: 1 }),
            ParserEvent::Char('c').localize(Snip { offset: 478, length: 1 },Snip { offset: 478, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 479, length: 1 },Snip { offset: 479, length: 1 }),
            ParserEvent::Char('n').localize(Snip { offset: 480, length: 1 },Snip { offset: 480, length: 1 }),
            ParserEvent::Char('s').localize(Snip { offset: 481, length: 1 },Snip { offset: 481, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 482, length: 1 },Snip { offset: 482, length: 1 }),
            ParserEvent::Char('r').localize(Snip { offset: 483, length: 1 },Snip { offset: 483, length: 1 }),
            ParserEvent::Char('u').localize(Snip { offset: 484, length: 1 },Snip { offset: 484, length: 1 }),
            ParserEvent::Char('c').localize(Snip { offset: 485, length: 1 },Snip { offset: 485, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 486, length: 1 },Snip { offset: 486, length: 1 }),
            ParserEvent::Char('i').localize(Snip { offset: 487, length: 1 },Snip { offset: 487, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 488, length: 1 },Snip { offset: 488, length: 1 }),
            ParserEvent::Char('n').localize(Snip { offset: 489, length: 1 },Snip { offset: 489, length: 1 }),
            ParserEvent::Char(' ').localize(Snip { offset: 490, length: 1 },Snip { offset: 490, length: 1 }),
            ParserEvent::Char('s').localize(Snip { offset: 491, length: 1 },Snip { offset: 491, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 492, length: 1 },Snip { offset: 492, length: 1 }),
            ParserEvent::Char('a').localize(Snip { offset: 493, length: 1 },Snip { offset: 493, length: 1 }),
            ParserEvent::Char('g').localize(Snip { offset: 494, length: 1 },Snip { offset: 494, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 495, length: 1 },Snip { offset: 495, length: 1 }),
            ParserEvent::Char('.').localize(Snip { offset: 496, length: 1 },Snip { offset: 496, length: 1 }),
            ParserEvent::Char('.').localize(Snip { offset: 497, length: 1 },Snip { offset: 497, length: 1 }),
            ParserEvent::Char('.').localize(Snip { offset: 498, length: 1 },Snip { offset: 498, length: 1 }),
            ParserEvent::Parsed(Tag {
                name: TagName::P, closing: Closing::Close, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 499, length: 1 },Snip { offset: 499, length: 1 }),
                end: ().localize(Snip { offset: 502, length: 1 },Snip { offset: 502, length: 1 }),
            }).localize(Snip { offset: 499, length: 4 },Snip { offset: 499, length: 4 }),
        ].into_iter();

        while let Some(local_event) = parser.next_event(&mut src).unwrap() {
            //let (local,event) = local_event.into_inner();
            //println!("ParserEvent::{:?}.localize({:?},{:?}),",event,local.chars(),local.bytes());
            match res_iter.next() {
                Some(ev) => {
                    println!("Parser: {:?}",local_event);
                    println!("Result: {:?}",ev);                    
                    assert_eq!(local_event,ev);
                },
                None => {
                    panic!("parser has more events then test result");
                },
            }
        }
    }

    
    #[test]
    fn a_img_2() {        
        let mut src = "
<p>In the common case, <a href=\"apis-in-html-documents.html#dynamic-markup-insertion\" title=\"dynamic markup
  insertion\">, e.g. using the <code title=\"dom-document-write\"><a href=\"apis-in-html-documents.html#dom-document-write\">document.write()</a></code> API.</p>
  <p><img alt=\"\" height=\"554\" src=\"https://dev.w3.org/html5/spec/images/parsing-model-overview.png\" width=\"427\"></p>
  <p id=\"nestedParsing\">There is only one set of states for the
  tokenizer stage and the tree construction stage...</p>".into_source().into_separator().merge_separators();
        let mut parser = Builder::new()
            .with_attribute(TagName::A,"href")
            .with_attribute(TagName::Img,"alt")
            .create();

        let mut res_iter = [
            ParserEvent::Breaker(Breaker::Line).localize(Snip { offset: 0, length: 1 },Snip { offset: 0, length: 1 }),
            ParserEvent::Parsed(Tag {
                name: TagName::P, closing: Closing::Open, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 1, length: 1 },Snip { offset: 1, length: 1 }),
                end: ().localize(Snip { offset: 3, length: 1 },Snip { offset: 3, length: 1 }),
            }).localize(Snip { offset: 1, length: 3 },Snip { offset: 1, length: 3 }),
            ParserEvent::Char('I').localize(Snip { offset: 4, length: 1 },Snip { offset: 4, length: 1 }),
            ParserEvent::Char('n').localize(Snip { offset: 5, length: 1 },Snip { offset: 5, length: 1 }),
            ParserEvent::Breaker(Breaker::Space).localize(Snip { offset: 6, length: 1 },Snip { offset: 6, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 7, length: 1 },Snip { offset: 7, length: 1 }),
            ParserEvent::Char('h').localize(Snip { offset: 8, length: 1 },Snip { offset: 8, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 9, length: 1 },Snip { offset: 9, length: 1 }),
            ParserEvent::Breaker(Breaker::Space).localize(Snip { offset: 10, length: 1 },Snip { offset: 10, length: 1 }),
            ParserEvent::Char('c').localize(Snip { offset: 11, length: 1 },Snip { offset: 11, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 12, length: 1 },Snip { offset: 12, length: 1 }),
            ParserEvent::Char('m').localize(Snip { offset: 13, length: 1 },Snip { offset: 13, length: 1 }),
            ParserEvent::Char('m').localize(Snip { offset: 14, length: 1 },Snip { offset: 14, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 15, length: 1 },Snip { offset: 15, length: 1 }),
            ParserEvent::Char('n').localize(Snip { offset: 16, length: 1 },Snip { offset: 16, length: 1 }),
            ParserEvent::Breaker(Breaker::Space).localize(Snip { offset: 17, length: 1 },Snip { offset: 17, length: 1 }),
            ParserEvent::Char('c').localize(Snip { offset: 18, length: 1 },Snip { offset: 18, length: 1 }),
            ParserEvent::Char('a').localize(Snip { offset: 19, length: 1 },Snip { offset: 19, length: 1 }),
            ParserEvent::Char('s').localize(Snip { offset: 20, length: 1 },Snip { offset: 20, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 21, length: 1 },Snip { offset: 21, length: 1 }),
            ParserEvent::Char(',').localize(Snip { offset: 22, length: 1 },Snip { offset: 22, length: 1 }),
            ParserEvent::Breaker(Breaker::Space).localize(Snip { offset: 23, length: 1 },Snip { offset: 23, length: 1 }),
            ParserEvent::Parsed(Tag {
                name: TagName::A, closing: Closing::Open,
                attributes: OptVec::One(("href".to_string(), Some(vec![
                    SourceEvent::Char('a').localize(Snip { offset: 33, length: 1 },Snip { offset: 33, length: 1 }),
                    SourceEvent::Char('p').localize(Snip { offset: 34, length: 1 },Snip { offset: 34, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 35, length: 1 },Snip { offset: 35, length: 1 }),
                    SourceEvent::Char('s').localize(Snip { offset: 36, length: 1 },Snip { offset: 36, length: 1 }),
                    SourceEvent::Char('-').localize(Snip { offset: 37, length: 1 },Snip { offset: 37, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 38, length: 1 },Snip { offset: 38, length: 1 }),
                    SourceEvent::Char('n').localize(Snip { offset: 39, length: 1 },Snip { offset: 39, length: 1 }),
                    SourceEvent::Char('-').localize(Snip { offset: 40, length: 1 },Snip { offset: 40, length: 1 }),
                    SourceEvent::Char('h').localize(Snip { offset: 41, length: 1 },Snip { offset: 41, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 42, length: 1 },Snip { offset: 42, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 43, length: 1 },Snip { offset: 43, length: 1 }),
                    SourceEvent::Char('l').localize(Snip { offset: 44, length: 1 },Snip { offset: 44, length: 1 }),
                    SourceEvent::Char('-').localize(Snip { offset: 45, length: 1 },Snip { offset: 45, length: 1 }),
                    SourceEvent::Char('d').localize(Snip { offset: 46, length: 1 },Snip { offset: 46, length: 1 }),
                    SourceEvent::Char('o').localize(Snip { offset: 47, length: 1 },Snip { offset: 47, length: 1 }),
                    SourceEvent::Char('c').localize(Snip { offset: 48, length: 1 },Snip { offset: 48, length: 1 }),
                    SourceEvent::Char('u').localize(Snip { offset: 49, length: 1 },Snip { offset: 49, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 50, length: 1 },Snip { offset: 50, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 51, length: 1 },Snip { offset: 51, length: 1 }),
                    SourceEvent::Char('n').localize(Snip { offset: 52, length: 1 },Snip { offset: 52, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 53, length: 1 },Snip { offset: 53, length: 1 }),
                    SourceEvent::Char('s').localize(Snip { offset: 54, length: 1 },Snip { offset: 54, length: 1 }),
                    SourceEvent::Char('.').localize(Snip { offset: 55, length: 1 },Snip { offset: 55, length: 1 }),
                    SourceEvent::Char('h').localize(Snip { offset: 56, length: 1 },Snip { offset: 56, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 57, length: 1 },Snip { offset: 57, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 58, length: 1 },Snip { offset: 58, length: 1 }),
                    SourceEvent::Char('l').localize(Snip { offset: 59, length: 1 },Snip { offset: 59, length: 1 }),
                    SourceEvent::Char('#').localize(Snip { offset: 60, length: 1 },Snip { offset: 60, length: 1 }),
                    SourceEvent::Char('d').localize(Snip { offset: 61, length: 1 },Snip { offset: 61, length: 1 }),
                    SourceEvent::Char('y').localize(Snip { offset: 62, length: 1 },Snip { offset: 62, length: 1 }),
                    SourceEvent::Char('n').localize(Snip { offset: 63, length: 1 },Snip { offset: 63, length: 1 }),
                    SourceEvent::Char('a').localize(Snip { offset: 64, length: 1 },Snip { offset: 64, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 65, length: 1 },Snip { offset: 65, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 66, length: 1 },Snip { offset: 66, length: 1 }),
                    SourceEvent::Char('c').localize(Snip { offset: 67, length: 1 },Snip { offset: 67, length: 1 }),
                    SourceEvent::Char('-').localize(Snip { offset: 68, length: 1 },Snip { offset: 68, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 69, length: 1 },Snip { offset: 69, length: 1 }),
                    SourceEvent::Char('a').localize(Snip { offset: 70, length: 1 },Snip { offset: 70, length: 1 }),
                    SourceEvent::Char('r').localize(Snip { offset: 71, length: 1 },Snip { offset: 71, length: 1 }),
                    SourceEvent::Char('k').localize(Snip { offset: 72, length: 1 },Snip { offset: 72, length: 1 }),
                    SourceEvent::Char('u').localize(Snip { offset: 73, length: 1 },Snip { offset: 73, length: 1 }),
                    SourceEvent::Char('p').localize(Snip { offset: 74, length: 1 },Snip { offset: 74, length: 1 }),
                    SourceEvent::Char('-').localize(Snip { offset: 75, length: 1 },Snip { offset: 75, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 76, length: 1 },Snip { offset: 76, length: 1 }),
                    SourceEvent::Char('n').localize(Snip { offset: 77, length: 1 },Snip { offset: 77, length: 1 }),
                    SourceEvent::Char('s').localize(Snip { offset: 78, length: 1 },Snip { offset: 78, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 79, length: 1 },Snip { offset: 79, length: 1 }),
                    SourceEvent::Char('r').localize(Snip { offset: 80, length: 1 },Snip { offset: 80, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 81, length: 1 },Snip { offset: 81, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 82, length: 1 },Snip { offset: 82, length: 1 }),
                    SourceEvent::Char('o').localize(Snip { offset: 83, length: 1 },Snip { offset: 83, length: 1 }),
                    SourceEvent::Char('n').localize(Snip { offset: 84, length: 1 },Snip { offset: 84, length: 1 }),
                ]))),                
                begin: ().localize(Snip { offset: 24, length: 1 },Snip { offset: 24, length: 1 }),
                end: ().localize(Snip { offset: 121, length: 1 },Snip { offset: 121, length: 1 }),                
            }).localize(Snip { offset: 24, length: 98 },Snip { offset: 24, length: 98 }),
            ParserEvent::Char(',').localize(Snip { offset: 122, length: 1 },Snip { offset: 122, length: 1 }),
            ParserEvent::Breaker(Breaker::Space).localize(Snip { offset: 123, length: 1 },Snip { offset: 123, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 124, length: 1 },Snip { offset: 124, length: 1 }),
            ParserEvent::Char('.').localize(Snip { offset: 125, length: 1 },Snip { offset: 125, length: 1 }),
            ParserEvent::Char('g').localize(Snip { offset: 126, length: 1 },Snip { offset: 126, length: 1 }),
            ParserEvent::Char('.').localize(Snip { offset: 127, length: 1 },Snip { offset: 127, length: 1 }),
            ParserEvent::Breaker(Breaker::Space).localize(Snip { offset: 128, length: 1 },Snip { offset: 128, length: 1 }),
            ParserEvent::Char('u').localize(Snip { offset: 129, length: 1 },Snip { offset: 129, length: 1 }),
            ParserEvent::Char('s').localize(Snip { offset: 130, length: 1 },Snip { offset: 130, length: 1 }),
            ParserEvent::Char('i').localize(Snip { offset: 131, length: 1 },Snip { offset: 131, length: 1 }),
            ParserEvent::Char('n').localize(Snip { offset: 132, length: 1 },Snip { offset: 132, length: 1 }),
            ParserEvent::Char('g').localize(Snip { offset: 133, length: 1 },Snip { offset: 133, length: 1 }),
            ParserEvent::Breaker(Breaker::Space).localize(Snip { offset: 134, length: 1 },Snip { offset: 134, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 135, length: 1 },Snip { offset: 135, length: 1 }),
            ParserEvent::Char('h').localize(Snip { offset: 136, length: 1 },Snip { offset: 136, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 137, length: 1 },Snip { offset: 137, length: 1 }),
            ParserEvent::Breaker(Breaker::Space).localize(Snip { offset: 138, length: 1 },Snip { offset: 138, length: 1 }),
            ParserEvent::Parsed(Tag {
                name: TagName::Code, closing: Closing::Open, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 139, length: 1 },Snip { offset: 139, length: 1 }),
                end: ().localize(Snip { offset: 171, length: 1 },Snip { offset: 171, length: 1 }),
            }).localize(Snip { offset: 139, length: 33 },Snip { offset: 139, length: 33 }),
            ParserEvent::Parsed(Tag {
                name: TagName::A, closing: Closing::Open,
                attributes: OptVec::One(("href".to_string(), Some(vec![
                    SourceEvent::Char('a').localize(Snip { offset: 181, length: 1 },Snip { offset: 181, length: 1 }),
                    SourceEvent::Char('p').localize(Snip { offset: 182, length: 1 },Snip { offset: 182, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 183, length: 1 },Snip { offset: 183, length: 1 }),
                    SourceEvent::Char('s').localize(Snip { offset: 184, length: 1 },Snip { offset: 184, length: 1 }),
                    SourceEvent::Char('-').localize(Snip { offset: 185, length: 1 },Snip { offset: 185, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 186, length: 1 },Snip { offset: 186, length: 1 }),
                    SourceEvent::Char('n').localize(Snip { offset: 187, length: 1 },Snip { offset: 187, length: 1 }),
                    SourceEvent::Char('-').localize(Snip { offset: 188, length: 1 },Snip { offset: 188, length: 1 }),
                    SourceEvent::Char('h').localize(Snip { offset: 189, length: 1 },Snip { offset: 189, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 190, length: 1 },Snip { offset: 190, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 191, length: 1 },Snip { offset: 191, length: 1 }),
                    SourceEvent::Char('l').localize(Snip { offset: 192, length: 1 },Snip { offset: 192, length: 1 }),
                    SourceEvent::Char('-').localize(Snip { offset: 193, length: 1 },Snip { offset: 193, length: 1 }),
                    SourceEvent::Char('d').localize(Snip { offset: 194, length: 1 },Snip { offset: 194, length: 1 }),
                    SourceEvent::Char('o').localize(Snip { offset: 195, length: 1 },Snip { offset: 195, length: 1 }),
                    SourceEvent::Char('c').localize(Snip { offset: 196, length: 1 },Snip { offset: 196, length: 1 }),
                    SourceEvent::Char('u').localize(Snip { offset: 197, length: 1 },Snip { offset: 197, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 198, length: 1 },Snip { offset: 198, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 199, length: 1 },Snip { offset: 199, length: 1 }),
                    SourceEvent::Char('n').localize(Snip { offset: 200, length: 1 },Snip { offset: 200, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 201, length: 1 },Snip { offset: 201, length: 1 }),
                    SourceEvent::Char('s').localize(Snip { offset: 202, length: 1 },Snip { offset: 202, length: 1 }),
                    SourceEvent::Char('.').localize(Snip { offset: 203, length: 1 },Snip { offset: 203, length: 1 }),
                    SourceEvent::Char('h').localize(Snip { offset: 204, length: 1 },Snip { offset: 204, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 205, length: 1 },Snip { offset: 205, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 206, length: 1 },Snip { offset: 206, length: 1 }),
                    SourceEvent::Char('l').localize(Snip { offset: 207, length: 1 },Snip { offset: 207, length: 1 }),
                    SourceEvent::Char('#').localize(Snip { offset: 208, length: 1 },Snip { offset: 208, length: 1 }),
                    SourceEvent::Char('d').localize(Snip { offset: 209, length: 1 },Snip { offset: 209, length: 1 }),
                    SourceEvent::Char('o').localize(Snip { offset: 210, length: 1 },Snip { offset: 210, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 211, length: 1 },Snip { offset: 211, length: 1 }),
                    SourceEvent::Char('-').localize(Snip { offset: 212, length: 1 },Snip { offset: 212, length: 1 }),
                    SourceEvent::Char('d').localize(Snip { offset: 213, length: 1 },Snip { offset: 213, length: 1 }),
                    SourceEvent::Char('o').localize(Snip { offset: 214, length: 1 },Snip { offset: 214, length: 1 }),
                    SourceEvent::Char('c').localize(Snip { offset: 215, length: 1 },Snip { offset: 215, length: 1 }),
                    SourceEvent::Char('u').localize(Snip { offset: 216, length: 1 },Snip { offset: 216, length: 1 }),
                    SourceEvent::Char('m').localize(Snip { offset: 217, length: 1 },Snip { offset: 217, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 218, length: 1 },Snip { offset: 218, length: 1 }),
                    SourceEvent::Char('n').localize(Snip { offset: 219, length: 1 },Snip { offset: 219, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 220, length: 1 },Snip { offset: 220, length: 1 }),
                    SourceEvent::Char('-').localize(Snip { offset: 221, length: 1 },Snip { offset: 221, length: 1 }),
                    SourceEvent::Char('w').localize(Snip { offset: 222, length: 1 },Snip { offset: 222, length: 1 }),
                    SourceEvent::Char('r').localize(Snip { offset: 223, length: 1 },Snip { offset: 223, length: 1 }),
                    SourceEvent::Char('i').localize(Snip { offset: 224, length: 1 },Snip { offset: 224, length: 1 }),
                    SourceEvent::Char('t').localize(Snip { offset: 225, length: 1 },Snip { offset: 225, length: 1 }),
                    SourceEvent::Char('e').localize(Snip { offset: 226, length: 1 },Snip { offset: 226, length: 1 }),
                ]))),                
                begin: ().localize(Snip { offset: 172, length: 1 },Snip { offset: 172, length: 1 }),
                end: ().localize(Snip { offset: 228, length: 1 },Snip { offset: 228, length: 1 }),
            }).localize(Snip { offset: 172, length: 57 },Snip { offset: 172, length: 57 }),
            ParserEvent::Char('d').localize(Snip { offset: 229, length: 1 },Snip { offset: 229, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 230, length: 1 },Snip { offset: 230, length: 1 }),
            ParserEvent::Char('c').localize(Snip { offset: 231, length: 1 },Snip { offset: 231, length: 1 }),
            ParserEvent::Char('u').localize(Snip { offset: 232, length: 1 },Snip { offset: 232, length: 1 }),
            ParserEvent::Char('m').localize(Snip { offset: 233, length: 1 },Snip { offset: 233, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 234, length: 1 },Snip { offset: 234, length: 1 }),
            ParserEvent::Char('n').localize(Snip { offset: 235, length: 1 },Snip { offset: 235, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 236, length: 1 },Snip { offset: 236, length: 1 }),
            ParserEvent::Char('.').localize(Snip { offset: 237, length: 1 },Snip { offset: 237, length: 1 }),
            ParserEvent::Char('w').localize(Snip { offset: 238, length: 1 },Snip { offset: 238, length: 1 }),
            ParserEvent::Char('r').localize(Snip { offset: 239, length: 1 },Snip { offset: 239, length: 1 }),
            ParserEvent::Char('i').localize(Snip { offset: 240, length: 1 },Snip { offset: 240, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 241, length: 1 },Snip { offset: 241, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 242, length: 1 },Snip { offset: 242, length: 1 }),
            ParserEvent::Char('(').localize(Snip { offset: 243, length: 1 },Snip { offset: 243, length: 1 }),
            ParserEvent::Char(')').localize(Snip { offset: 244, length: 1 },Snip { offset: 244, length: 1 }),
            ParserEvent::Parsed(Tag {
                name: TagName::A, closing: Closing::Close, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 245, length: 1 },Snip { offset: 245, length: 1 }),
                end: ().localize(Snip { offset: 248, length: 1 },Snip { offset: 248, length: 1 }),
            }).localize(Snip { offset: 245, length: 4 },Snip { offset: 245, length: 4 }),
            ParserEvent::Parsed(Tag {
                name: TagName::Code, closing: Closing::Close, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 249, length: 1 },Snip { offset: 249, length: 1 }),
                end: ().localize(Snip { offset: 255, length: 1 },Snip { offset: 255, length: 1 }),
            }).localize(Snip { offset: 249, length: 7 },Snip { offset: 249, length: 7 }),
            ParserEvent::Breaker(Breaker::Space).localize(Snip { offset: 256, length: 1 },Snip { offset: 256, length: 1 }),
            ParserEvent::Char('A').localize(Snip { offset: 257, length: 1 },Snip { offset: 257, length: 1 }),
            ParserEvent::Char('P').localize(Snip { offset: 258, length: 1 },Snip { offset: 258, length: 1 }),
            ParserEvent::Char('I').localize(Snip { offset: 259, length: 1 },Snip { offset: 259, length: 1 }),
            ParserEvent::Char('.').localize(Snip { offset: 260, length: 1 },Snip { offset: 260, length: 1 }),
            ParserEvent::Parsed(Tag {
                name: TagName::P, closing: Closing::Close, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 261, length: 1 },Snip { offset: 261, length: 1 }),
                end: ().localize(Snip { offset: 264, length: 1 },Snip { offset: 264, length: 1 }),
            }).localize(Snip { offset: 261, length: 4 },Snip { offset: 261, length: 4 }),
            ParserEvent::Breaker(Breaker::Line).localize(Snip { offset: 265, length: 1 },Snip { offset: 265, length: 1 }),
            ParserEvent::Parsed(Tag {
                name: TagName::P, closing: Closing::Open, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 268, length: 1 },Snip { offset: 268, length: 1 }),
                end: ().localize(Snip { offset: 270, length: 1 },Snip { offset: 270, length: 1 }),
            }).localize(Snip { offset: 268, length: 3 },Snip { offset: 268, length: 3 }),
            ParserEvent::Parsed(Tag {
                name: TagName::Img, closing: Closing::Void,
                attributes: OptVec::One(("alt".to_string(), Some(Vec::new()))),
                begin: ().localize(Snip { offset: 271, length: 1 },Snip { offset: 271, length: 1 }),
                end: ().localize(Snip { offset: 377, length: 1 },Snip { offset: 377, length: 1 }),
            }).localize(Snip { offset: 271, length: 107 },Snip { offset: 271, length: 107 }),
            ParserEvent::Parsed(Tag {
                name: TagName::P, closing: Closing::Close, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 378, length: 1 },Snip { offset: 378, length: 1 }),
                end: ().localize(Snip { offset: 381, length: 1 },Snip { offset: 381, length: 1 }),
            }).localize(Snip { offset: 378, length: 4 },Snip { offset: 378, length: 4 }),
            ParserEvent::Breaker(Breaker::Line).localize(Snip { offset: 382, length: 1 },Snip { offset: 382, length: 1 }),
            ParserEvent::Parsed(Tag {
                name: TagName::P, closing: Closing::Open, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 385, length: 1 },Snip { offset: 385, length: 1 }),
                end: ().localize(Snip { offset: 406, length: 1 },Snip { offset: 406, length: 1 }),
            }).localize(Snip { offset: 385, length: 22 },Snip { offset: 385, length: 22 }),
            ParserEvent::Char('T').localize(Snip { offset: 407, length: 1 },Snip { offset: 407, length: 1 }),
            ParserEvent::Char('h').localize(Snip { offset: 408, length: 1 },Snip { offset: 408, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 409, length: 1 },Snip { offset: 409, length: 1 }),
            ParserEvent::Char('r').localize(Snip { offset: 410, length: 1 },Snip { offset: 410, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 411, length: 1 },Snip { offset: 411, length: 1 }),
            ParserEvent::Breaker(Breaker::Space).localize(Snip { offset: 412, length: 1 },Snip { offset: 412, length: 1 }),
            ParserEvent::Char('i').localize(Snip { offset: 413, length: 1 },Snip { offset: 413, length: 1 }),
            ParserEvent::Char('s').localize(Snip { offset: 414, length: 1 },Snip { offset: 414, length: 1 }),
            ParserEvent::Breaker(Breaker::Space).localize(Snip { offset: 415, length: 1 },Snip { offset: 415, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 416, length: 1 },Snip { offset: 416, length: 1 }),
            ParserEvent::Char('n').localize(Snip { offset: 417, length: 1 },Snip { offset: 417, length: 1 }),
            ParserEvent::Char('l').localize(Snip { offset: 418, length: 1 },Snip { offset: 418, length: 1 }),
            ParserEvent::Char('y').localize(Snip { offset: 419, length: 1 },Snip { offset: 419, length: 1 }),
            ParserEvent::Breaker(Breaker::Space).localize(Snip { offset: 420, length: 1 },Snip { offset: 420, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 421, length: 1 },Snip { offset: 421, length: 1 }),
            ParserEvent::Char('n').localize(Snip { offset: 422, length: 1 },Snip { offset: 422, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 423, length: 1 },Snip { offset: 423, length: 1 }),
            ParserEvent::Breaker(Breaker::Space).localize(Snip { offset: 424, length: 1 },Snip { offset: 424, length: 1 }),
            ParserEvent::Char('s').localize(Snip { offset: 425, length: 1 },Snip { offset: 425, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 426, length: 1 },Snip { offset: 426, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 427, length: 1 },Snip { offset: 427, length: 1 }),
            ParserEvent::Breaker(Breaker::Space).localize(Snip { offset: 428, length: 1 },Snip { offset: 428, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 429, length: 1 },Snip { offset: 429, length: 1 }),
            ParserEvent::Char('f').localize(Snip { offset: 430, length: 1 },Snip { offset: 430, length: 1 }),
            ParserEvent::Breaker(Breaker::Space).localize(Snip { offset: 431, length: 1 },Snip { offset: 431, length: 1 }),
            ParserEvent::Char('s').localize(Snip { offset: 432, length: 1 },Snip { offset: 432, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 433, length: 1 },Snip { offset: 433, length: 1 }),
            ParserEvent::Char('a').localize(Snip { offset: 434, length: 1 },Snip { offset: 434, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 435, length: 1 },Snip { offset: 435, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 436, length: 1 },Snip { offset: 436, length: 1 }),
            ParserEvent::Char('s').localize(Snip { offset: 437, length: 1 },Snip { offset: 437, length: 1 }),
            ParserEvent::Breaker(Breaker::Space).localize(Snip { offset: 438, length: 1 },Snip { offset: 438, length: 1 }),
            ParserEvent::Char('f').localize(Snip { offset: 439, length: 1 },Snip { offset: 439, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 440, length: 1 },Snip { offset: 440, length: 1 }),
            ParserEvent::Char('r').localize(Snip { offset: 441, length: 1 },Snip { offset: 441, length: 1 }),
            ParserEvent::Breaker(Breaker::Space).localize(Snip { offset: 442, length: 1 },Snip { offset: 442, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 443, length: 1 },Snip { offset: 443, length: 1 }),
            ParserEvent::Char('h').localize(Snip { offset: 444, length: 1 },Snip { offset: 444, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 445, length: 1 },Snip { offset: 445, length: 1 }),
            ParserEvent::Breaker(Breaker::Line).localize(Snip { offset: 446, length: 1 },Snip { offset: 446, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 449, length: 1 },Snip { offset: 449, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 450, length: 1 },Snip { offset: 450, length: 1 }),
            ParserEvent::Char('k').localize(Snip { offset: 451, length: 1 },Snip { offset: 451, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 452, length: 1 },Snip { offset: 452, length: 1 }),
            ParserEvent::Char('n').localize(Snip { offset: 453, length: 1 },Snip { offset: 453, length: 1 }),
            ParserEvent::Char('i').localize(Snip { offset: 454, length: 1 },Snip { offset: 454, length: 1 }),
            ParserEvent::Char('z').localize(Snip { offset: 455, length: 1 },Snip { offset: 455, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 456, length: 1 },Snip { offset: 456, length: 1 }),
            ParserEvent::Char('r').localize(Snip { offset: 457, length: 1 },Snip { offset: 457, length: 1 }),
            ParserEvent::Breaker(Breaker::Space).localize(Snip { offset: 458, length: 1 },Snip { offset: 458, length: 1 }),
            ParserEvent::Char('s').localize(Snip { offset: 459, length: 1 },Snip { offset: 459, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 460, length: 1 },Snip { offset: 460, length: 1 }),
            ParserEvent::Char('a').localize(Snip { offset: 461, length: 1 },Snip { offset: 461, length: 1 }),
            ParserEvent::Char('g').localize(Snip { offset: 462, length: 1 },Snip { offset: 462, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 463, length: 1 },Snip { offset: 463, length: 1 }),
            ParserEvent::Breaker(Breaker::Space).localize(Snip { offset: 464, length: 1 },Snip { offset: 464, length: 1 }),
            ParserEvent::Char('a').localize(Snip { offset: 465, length: 1 },Snip { offset: 465, length: 1 }),
            ParserEvent::Char('n').localize(Snip { offset: 466, length: 1 },Snip { offset: 466, length: 1 }),
            ParserEvent::Char('d').localize(Snip { offset: 467, length: 1 },Snip { offset: 467, length: 1 }),
            ParserEvent::Breaker(Breaker::Space).localize(Snip { offset: 468, length: 1 },Snip { offset: 468, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 469, length: 1 },Snip { offset: 469, length: 1 }),
            ParserEvent::Char('h').localize(Snip { offset: 470, length: 1 },Snip { offset: 470, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 471, length: 1 },Snip { offset: 471, length: 1 }),
            ParserEvent::Breaker(Breaker::Space).localize(Snip { offset: 472, length: 1 },Snip { offset: 472, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 473, length: 1 },Snip { offset: 473, length: 1 }),
            ParserEvent::Char('r').localize(Snip { offset: 474, length: 1 },Snip { offset: 474, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 475, length: 1 },Snip { offset: 475, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 476, length: 1 },Snip { offset: 476, length: 1 }),
            ParserEvent::Breaker(Breaker::Space).localize(Snip { offset: 477, length: 1 },Snip { offset: 477, length: 1 }),
            ParserEvent::Char('c').localize(Snip { offset: 478, length: 1 },Snip { offset: 478, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 479, length: 1 },Snip { offset: 479, length: 1 }),
            ParserEvent::Char('n').localize(Snip { offset: 480, length: 1 },Snip { offset: 480, length: 1 }),
            ParserEvent::Char('s').localize(Snip { offset: 481, length: 1 },Snip { offset: 481, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 482, length: 1 },Snip { offset: 482, length: 1 }),
            ParserEvent::Char('r').localize(Snip { offset: 483, length: 1 },Snip { offset: 483, length: 1 }),
            ParserEvent::Char('u').localize(Snip { offset: 484, length: 1 },Snip { offset: 484, length: 1 }),
            ParserEvent::Char('c').localize(Snip { offset: 485, length: 1 },Snip { offset: 485, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 486, length: 1 },Snip { offset: 486, length: 1 }),
            ParserEvent::Char('i').localize(Snip { offset: 487, length: 1 },Snip { offset: 487, length: 1 }),
            ParserEvent::Char('o').localize(Snip { offset: 488, length: 1 },Snip { offset: 488, length: 1 }),
            ParserEvent::Char('n').localize(Snip { offset: 489, length: 1 },Snip { offset: 489, length: 1 }),
            ParserEvent::Breaker(Breaker::Space).localize(Snip { offset: 490, length: 1 },Snip { offset: 490, length: 1 }),
            ParserEvent::Char('s').localize(Snip { offset: 491, length: 1 },Snip { offset: 491, length: 1 }),
            ParserEvent::Char('t').localize(Snip { offset: 492, length: 1 },Snip { offset: 492, length: 1 }),
            ParserEvent::Char('a').localize(Snip { offset: 493, length: 1 },Snip { offset: 493, length: 1 }),
            ParserEvent::Char('g').localize(Snip { offset: 494, length: 1 },Snip { offset: 494, length: 1 }),
            ParserEvent::Char('e').localize(Snip { offset: 495, length: 1 },Snip { offset: 495, length: 1 }),
            ParserEvent::Char('.').localize(Snip { offset: 496, length: 1 },Snip { offset: 496, length: 1 }),
            ParserEvent::Char('.').localize(Snip { offset: 497, length: 1 },Snip { offset: 497, length: 1 }),
            ParserEvent::Char('.').localize(Snip { offset: 498, length: 1 },Snip { offset: 498, length: 1 }),
            ParserEvent::Parsed(Tag {
                name: TagName::P, closing: Closing::Close, attributes: OptVec::None,
                begin: ().localize(Snip { offset: 499, length: 1 },Snip { offset: 499, length: 1 }),
                end: ().localize(Snip { offset: 502, length: 1 },Snip { offset: 502, length: 1 }),
            }).localize(Snip { offset: 499, length: 4 },Snip { offset: 499, length: 4 }),
        ].into_iter();

        while let Some(local_event) = parser.next_event(&mut src).unwrap() {
            //let (local,event) = local_event.into_inner();
            //println!("ParserEvent::{:?}.localize({:?},{:?}),",event,local.chars(),local.bytes());
            match res_iter.next() {
                Some(ev) => {
                    println!("Parser: {:?}",local_event);
                    println!("Result: {:?}",ev);
                    /*if let ParserEvent::Parsed(tag) = local_event.data() {
                        for (_,attr) in &tag.attributes {
                            if let Some(attr) = attr {
                                println!("[");
                                for lse in attr {
                                    println!("SourceEvent::{:?}.localize({:?},{:?}),",lse.data(),lse.chars(),lse.bytes());
                                }
                                println!("]");
                            }
                        }
                        println!("begin: ().localize({:?},{:?}),",tag.begin.chars(),tag.begin.bytes());
                        println!("end: ().localize({:?},{:?}),",tag.end.chars(),tag.end.bytes());
                    }*/
                    assert_eq!(local_event,ev);
                },
                None => {
                    panic!("parser has more events then test result");
                },
            }
        }
    }

    /*#[test]
    fn basic_pipe() {
        let mut src = "<h1>Hello, world!</h1>Привет, мир, &#x2<wbr>764;!"
            .into_source()
            .pipe(Builder::new().create().piped(|t: Tag| {
                // skip all tags
                Some(SourceEvent::Breaker(Breaker::Word))
            }));

        while let Some(local_se) = src.next_char().unwrap() {
            println!("{:?}",local_se);
        }
        panic!();
    }

    #[test]
    fn basic_pipe_ent() {
        let mut src = "<h1>Hello, world!</h1>Привет, мир, &#x2<wbr>764;!"
            .into_source()
            .pipe(crate::tagger::Builder::new().create().piped(|t: Tag| {
                Some(match t.name {
                    TagName::Wbr => SourceEvent::Breaker(Breaker::None),
                    _ => SourceEvent::Breaker(Breaker::Word),
                })
            }))
            .pipe(crate::entities::Builder::new().create().into_piped());

        while let Some(local_se) = src.next_char().unwrap() {
            println!("{:?}",local_se);
        }
        panic!();
    }
     */
        
}
