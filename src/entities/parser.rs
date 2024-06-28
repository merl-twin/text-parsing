use super::{
    entities::{Entity,Instance},
    state::EntityState,
};
use crate::{
    ParserEvent,Error,Local,ParserResult,
    Source, SourceNext, FlatParser,
    ParserNext, InnerParser,
};

/*

  Entities: https://www.w3.org/TR/html5/entities.json


    Semicolon may be optional (html legacy)

    &lt;   &#60;  &#x3C;

    "&" + Name + ";"  

    // without parametr entities [ %name; ]  ????

	NameStartChar	   ::=   	":" | [A-Z] | "_" | [a-z] | [#xC0-#xD6] | [#xD8-#xF6] | [#xF8-#x2FF] | [#x370-#x37D] | [#x37F-#x1FFF] | [#x200C-#x200D] | [#x2070-#x218F] | [#x2C00-#x2FEF] | [#x3001-#xD7FF] | [#xF900-#xFDCF] | [#xFDF0-#xFFFD] | [#x10000-#xEFFFF]
   	NameChar	   ::=   	NameStartChar | "-" | "." | [0-9] | #xB7 | [#x0300-#x036F] | [#x203F-#x2040]
   	Name	   ::=   	NameStartChar (NameChar)*

*/



#[derive(Debug,Clone)]
pub struct Builder {

}
impl Builder {
    pub fn new() -> Builder {
        Builder { }
    }
    pub fn create(self) -> Parser {
        Parser(InnerParser::new(()))
    }
}

pub struct Parser(InnerParser<EntityState,Entity,()>);
impl ParserNext for Parser {
    type Data = Entity;
    
    fn next_event<S: Source>(&mut self, src: &mut S) -> ParserResult<Entity> {
        self.0.next_event(src)
    }
}

/*impl FlatParser for Parser {
    type Flatten = SourceParser;
    
    fn flatten(self) -> Self::Flatten {
        SourceParser {
            parser: self.0,
            tmp_char: None,
        }
    }
}

pub struct SourceParser {
    parser: InnerParser<EntityState,Entity,()>,
    tmp_char: Option<Local<char>>,
}

impl SourceNext for SourceParser {
    fn next_event<S: Source>(&mut self, src: &mut S) -> Result<Option<Local<char>>,Error> {
        Ok(match self.tmp_char.take() {
            Some(local_char) => Some(local_char), 
            None => match self.parser.next_event(src)? {
                Some(local_ent) => match local_ent.data() {
                    ParserEvent::Char(c) => Some(local_ent.local(*c)),
                    ParserEvent::Breaker(b) => //Some(local_ent.local(b)),
                    ParserEvent::Parsed(ent) => match ent.entity {
                        Instance::Char(c) => Some(local_ent.local(c)),
                        Instance::Char2(c1,c2) => {
                            self.tmp_char = Some(local_ent.local(c2));
                            Some(local_ent.local(c1))
                        },
                    },
                },
                None => None,
            },
        })
    }
}*/


#[cfg(test)]
mod tests {
    use crate::*;
    use super::*;
    
    #[test]
    fn basic() {
        let mut src = " &blabla; &#111111111; &quot &AMP; &&GreaterGreater; &#128175; &#x2764;".into_source();
        let mut parser = Builder::new().create();

        while let Some(local_event) = parser.next_event(&mut src).unwrap() {
            println!("{:?}",local_event);
        }
        panic!();
    }

    #[test]
    fn basic_flatten() {
        let mut src = " &blabla; &#111111111; &quot &AMP; &&GreaterGreater; &#128175; &#x2764;".into_source();
        let mut parser = Builder::new().create().flatten();

        while let Some(local_event) = parser.next_event(&mut src).unwrap() {
            println!("{:?}",local_event);
        }
        panic!();
    } 
}

