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
    ParserNext, InnerParser,
};

/*

  Algorithm: https://dev.w3.org/html5/spec-LC/parsing.html

  Entities: https://www.w3.org/TR/html5/entities.json

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
    pub fn create(self) -> Parser {
        Parser(InnerParser::new(self.properties))
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

pub struct Parser(InnerParser<TaggerState,Tag,TaggerProperties>);
impl ParserNext for Parser {
    type Data = Tag;
    
    fn next_event<S: Source>(&mut self, src: &mut S) -> ParserResult<Tag> {
        self.0.next_event(src)
    }
}



#[cfg(test)]
mod tests {
    use crate::*;
    use super::*;
    
    #[test]
    fn basic() {
        let mut src = "<h1>Hello, world!</h1>Привет, мир!".into_source();
        let mut parser = Builder::new().create();

        while let Some(local_event) = parser.next_event(&mut src).unwrap() {
            println!("{:?}",local_event);
        }
        panic!();
    }

    #[test]
    fn a_img() {
        let mut src = "
<p>In the common case, the data handled by the tokenization stage
  comes from the network, but <a href=\"apis-in-html-documents.html#dynamic-markup-insertion\" title=\"dynamic markup
  insertion\">it can also come from script</a> running in the user
  agent, e.g. using the <code title=\"dom-document-write\"><a href=\"apis-in-html-documents.html#dom-document-write\">document.write()</a></code> API.</p>

  <p><img alt=\"\" height=\"554\" src=\"https://dev.w3.org/html5/spec/images/parsing-model-overview.png\" width=\"427\"></p>

  <p id=\"nestedParsing\">There is only one set of states for the
  tokenizer stage and the tree construction stage...</p>".into_source();
        let mut parser = Builder::new()
            .with_attribute(TagName::A,"href")
            .with_attribute(TagName::Img,"alt")
            .create();

        while let Some(local_event) = parser.next_event(&mut src).unwrap() {
            println!("{:?}",local_event);
        }
        panic!();
    }
        
}