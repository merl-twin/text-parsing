
use opt_struct::OptVec;


#[derive(Debug,Eq,PartialEq)]
pub struct Tag {
    pub name: TagName,
    //pub breaker: Breaker,
    pub closing: Closing,
    pub attributes: OptVec<(String, Option<String>)>,
}



impl Tag {
   /* pub fn from_slice(s: &str) -> Tag {
        
}*/
    pub fn new(tag: TagName, clo: Closing, attrs: OptVec<(String, Option<String>)>) -> Tag {
        Tag {
            //breaker: tag.breaker(),
            closing: match tag.is_void() {
                true => Closing::Void,
                false => clo,
            },
            name: tag,            
            attributes: attrs,
        }
    }
}

/*

Void elements
area, base, br, col, command, embed, hr, img, input, keygen, link, meta, param, source, track, wbr

7.1. Flow elements
a p hr pre ul ol dl div h1 h2 h3 h4 h5 h6 hgroup address blockquote ins del object map noscript section nav article aside header footer video audio figure table form fieldset menu canvas details

7.2. Metadata elements
link style meta name script noscript command

7.3. Phrasing elements
a em strong small mark abbr dfn i b s code var samp kbd sup sub q cite span bdo bdi br wbr ins del img embed object iframe map area script noscript ruby video audio input textarea select button label output datalist keygen progress command canvas time meter

Basic HTML
Tag	Description
<!DOCTYPE> 	Defines the document type
<html>	Defines an HTML document
<head>	Contains metadata/information for the document
<title>	Defines a title for the document
<body>	Defines the document's body
<h1> to <h6>	Defines HTML headings
<p>	Defines a paragraph
<br>	Inserts a single line break
<hr>	Defines a thematic change in the content
<!--...-->	Defines a comment
Formatting
Tag	Description
<acronym>	Not supported in HTML5. Use <abbr> instead.
Defines an acronym
<abbr>	Defines an abbreviation or an acronym
<address>	Defines contact information for the author/owner of a document/article
<b>	Defines bold text
<bdi>	Isolates a part of text that might be formatted in a different direction from other text outside it
<bdo>	Overrides the current text direction
<big>	Not supported in HTML5. Use CSS instead.
Defines big text
<blockquote>	Defines a section that is quoted from another source
<center>	Not supported in HTML5. Use CSS instead.
Defines centered text
<cite>	Defines the title of a work
<code>	Defines a piece of computer code
<del>	Defines text that has been deleted from a document
<dfn>	Specifies a term that is going to be defined within the content
<em>	Defines emphasized text 
<font>	Not supported in HTML5. Use CSS instead.
Defines font, color, and size for text
<i>	Defines a part of text in an alternate voice or mood
<ins>	Defines a text that has been inserted into a document
<kbd>	Defines keyboard input
<mark>	Defines marked/highlighted text
<meter>	Defines a scalar measurement within a known range (a gauge)
<pre>	Defines preformatted text
<progress>	Represents the progress of a task
<q>	Defines a short quotation
<rp>	Defines what to show in browsers that do not support ruby annotations
<rt>	Defines an explanation/pronunciation of characters (for East Asian typography)
<ruby>	Defines a ruby annotation (for East Asian typography)
<s>	Defines text that is no longer correct
<samp>	Defines sample output from a computer program
<small>	Defines smaller text
<strike>	Not supported in HTML5. Use <del> or <s> instead.
Defines strikethrough text
<strong>	Defines important text
<sub>	Defines subscripted text
<sup>	Defines superscripted text
<template>	Defines a container for content that should be hidden when the page loads
<time>	Defines a specific time (or datetime)
<tt>	Not supported in HTML5. Use CSS instead.
Defines teletype text
<u>	Defines some text that is unarticulated and styled differently from normal text
<var>	Defines a variable
<wbr>	Defines a possible line-break

Forms and Input
Tag	Description
<form>	Defines an HTML form for user input
<input>	Defines an input control
<textarea>	Defines a multiline input control (text area)
<button>	Defines a clickable button
<select>	Defines a drop-down list
<optgroup>	Defines a group of related options in a drop-down list
<option>	Defines an option in a drop-down list
<label>	Defines a label for an <input> element
<fieldset>	Groups related elements in a form
<legend>	Defines a caption for a <fieldset> element
<datalist>	Specifies a list of pre-defined options for input controls
<output>	Defines the result of a calculation

Frames
Tag	Description
<frame>	Not supported in HTML5.
Defines a window (a frame) in a frameset
<frameset>	Not supported in HTML5.
Defines a set of frames
<noframes>	Not supported in HTML5.
Defines an alternate content for users that do not support frames
<iframe>	Defines an inline frame

Images
Tag	Description
<img>	Defines an image
<map>	Defines a client-side image map
<area>	Defines an area inside an image map
<canvas>	Used to draw graphics, on the fly, via scripting (usually JavaScript)
<figcaption>	Defines a caption for a <figure> element
<figure>	Specifies self-contained content
<picture>	Defines a container for multiple image resources
<svg>	Defines a container for SVG graphics

Audio / Video
Tag	Description
<audio>	Defines sound content
<source>	Defines multiple media resources for media elements (<video>, <audio> and <picture>)
<track>	Defines text tracks for media elements (<video> and <audio>)
<video>	Defines a video or movie
Links
Tag	Description
<a>	Defines a hyperlink
<link>	Defines the relationship between a document and an external resource (most used to link to style sheets)
<nav>	Defines navigation links

Lists
Tag	Description
<menu>	Defines an alternative unordered list
<ul>	Defines an unordered list
<ol>	Defines an ordered list
<li>	Defines a list item
<dir>	Not supported in HTML5. Use <ul> instead.
Defines a directory list
<dl>	Defines a description list
<dt>	Defines a term/name in a description list
<dd>	Defines a description of a term/name in a description list

Tables
Tag	Description
<table>	Defines a table
<caption>	Defines a table caption
<th>	Defines a header cell in a table
<tr>	Defines a row in a table
<td>	Defines a cell in a table
<thead>	Groups the header content in a table
<tbody>	Groups the body content in a table
<tfoot>	Groups the footer content in a table
<col>	Specifies column properties for each column within a <colgroup> element
<colgroup>	Specifies a group of one or more columns in a table for formatting

Styles and Semantics
Tag	Description
<style>	Defines style information for a document
<div>	Defines a section in a document
<span>	Defines a section in a document
<header>	Defines a header for a document or section
<hgroup>	Defines a header and related content
<footer>	Defines a footer for a document or section
<main>	Specifies the main content of a document
<section>	Defines a section in a document
<search>	Defines a search section
<article>	Defines an article
<aside>	Defines content aside from the page content
<details>	Defines additional details that the user can view or hide
<dialog>	Defines a dialog box or window
<summary>	Defines a visible heading for a <details> element
<data>	Adds a machine-readable translation of a given content

Meta Info
Tag	Description
<meta>	Defines metadata about an HTML document
<base>	Specifies the base URL/target for all relative URLs in a document
<basefont>	Not supported in HTML5. Use CSS instead.
Specifies a default color, size, and font for all text in a document

Programming
Tag	Description
<script>	Defines a client-side script
<noscript>	Defines an alternate content for users that do not support client-side scripts
<applet>	Not supported in HTML5. Use <embed> or <object> instead.
Defines an embedded applet
<embed>	Defines a container for an external (non-HTML) application
<object>	Defines an embedded object
<param>	Defines a parameter for an object

*/

#[derive(Debug,Clone,Copy,Eq,PartialEq)]
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
    // Basic HTML
    Html,
    Head,
    Title,
    Body,
    H1,
    H2,
    H3,
    H4,
    H5,
    H6,
    P,
    Br,
    Hr,

    //Styles and Semantics
    Style,
    Div,
    Span,
    Header,
    Hgroup,
    Footer,
    Main,
    Section,
    Search,
    Article,
    Aside,
    Details,
    Dialog,
    Summary,
    Data,

    // Formatting
    Acronym,
    Abbr,
    Address,
    B,
    Bdi,
    Bdo,
    Big,
    Blockquote,
    Center,
    Cite,
    Code,
    Del,
    Dfn,
    Em,
    Font,
    I,
    Ins,
    Kbd,
    Mark,
    Meter,
    Pre,
    Progress,
    Q,
    Rp,
    Rt,
    Ruby,
    S,
    Samp,
    Small,
    Strike,
    Strong,
    Sub,
    Sup,
    Template,
    Time,
    Tt,
    U,
    Var,
    Wbr,

    //Links
    A,
    Link,
    Nav,
    
    //Lists
    Menu,
    Ul,
    Ol,
    Li,
    Dir,
    Dl,
    Dt,
    Dd,

    //Tables
    Table,
    Caption,
    Th,
    Tr,
    Td,
    Thead,
    Tbody,
    Tfoot,
    Col,
    Colgroup,

    //Images
    Img,
    Map,
    Area,
    Canvas,
    Figcaption,
    Figure,
    Picture,
    Svg,
        
    // Forms and Input
    Form,
    Input,
    Textarea,
    Button,
    Select,
    Optgroup,
    Option,
    Label,
    Fieldset,
    Legend,
    Datalist,
    Output,
        
    // Frames
    Frame,
    Frameset,
    Noframes,
    Iframe,
    
    //Audio / Video
    Audio,
    Source,
    Track,
    Video,

    // Meta Info
    Meta,
    Base,
    Basefont,

    // Programming
    Script,
    Noscript,
    Applet,
    Embed,
    Object,
    Param,

    Command,
    Keygen,

    X(SpecTag),
    Other(String),
}
impl TagName {
    pub fn from(s: String) -> TagName {
        match &s as &str {
            // Basic HTML
            "html" => TagName::Html,
            "head" => TagName::Head,
            "title" => TagName::Title,
            "body" => TagName::Body,
            "h1" => TagName::H1,
            "h2" => TagName::H2,
            "h3" => TagName::H3,
            "h4" => TagName::H4,
            "h5" => TagName::H5,
            "h6" => TagName::H6,
            "p" => TagName::P,
            "br" => TagName::Br,
            "hr" => TagName::Hr,
            
            // Formatting
            "acronym" => TagName::Acronym,
            "abbr" => TagName::Abbr,
            "address" => TagName::Address,
            "b" => TagName::B,
            "bdi" => TagName::Bdi,
            "bdo" => TagName::Bdo,
            "big" => TagName::Big,
            "blockquote" => TagName::Blockquote,
            "center" => TagName::Center,
            "cite" => TagName::Cite,
            "code" => TagName::Code,
            "del" => TagName::Del,
            "dfn" => TagName::Dfn,
            "em" => TagName::Em,
            "font" => TagName::Font,
            "i" => TagName::I,
            "ins" => TagName::Ins,
            "kbd" => TagName::Kbd,
            "mark" => TagName::Mark,
            "meter" => TagName::Meter,
            "pre" => TagName::Pre,
            "progress" => TagName::Progress,
            "q" => TagName::Q,
            "rp" => TagName::Rp,
            "rt" => TagName::Rt,
            "ruby" => TagName::Ruby,
            "s" => TagName::S,
            "samp" => TagName::Samp,
            "small" => TagName::Small,
            "strike" => TagName::Strike,
            "strong" => TagName::Strong,
            "sub" => TagName::Sub,
            "sup" => TagName::Sup,
            "template" => TagName::Template,
            "time" => TagName::Time,
            "tt" => TagName::Tt,
            "u" => TagName::U,
            "var" => TagName::Var,
            "wbr" => TagName::Wbr,
            
            // Forms and Input
            "form" => TagName::Form,
            "input" => TagName::Input,
            "textarea" => TagName::Textarea,
            "button" => TagName::Button,
            "select" => TagName::Select,
            "optgroup" => TagName::Optgroup,
            "option" => TagName::Option,
            "label" => TagName::Label,
            "fieldset" => TagName::Fieldset,
            "legend" => TagName::Legend,
            "datalist" => TagName::Datalist,
            "output" => TagName::Output,
            
            // Frames
            "frame" => TagName::Frame,
            "frameset" => TagName::Frameset,
            "noframes" => TagName::Noframes,
            "iframe" => TagName::Iframe,
            
            //Images
            "img" => TagName::Img,
            "map" => TagName::Map,
            "area" => TagName::Area,
            "canvas" => TagName::Canvas,
            "figcaption" => TagName::Figcaption,
            "figure" => TagName::Figure,
            "picture" => TagName::Picture,
            "svg" => TagName::Svg,
            
            //Audio / Video
            "audio" => TagName::Audio,
            "source" => TagName::Source,
            "track" => TagName::Track,
            "video" => TagName::Video,
            
            //Links
            "a" => TagName::A,
            "link" => TagName::Link,
            "nav" => TagName::Nav,
            
            //Lists
            "menu" => TagName::Menu,
            "ul" => TagName::Ul,
            "ol" => TagName::Ol,
            "li" => TagName::Li,
            "dir" => TagName::Dir,
            "dl" => TagName::Dl,
            "dt" => TagName::Dt,
            "dd" => TagName::Dd,

            //Tables
            "table" => TagName::Table,
            "caption" => TagName::Caption,
            "th" => TagName::Th,
            "tr" => TagName::Tr,
            "td" => TagName::Td,
            "thead" => TagName::Thead,
            "tbody" => TagName::Tbody,
            "tfoot" => TagName::Tfoot,
            "col" => TagName::Col,
            "colgroup" => TagName::Colgroup,

            //Styles and Semantics
            "style" => TagName::Style,
            "div" => TagName::Div,
            "span" => TagName::Span,
            "header" => TagName::Header,
            "hgroup" => TagName::Hgroup,
            "footer" => TagName::Footer,
            "main" => TagName::Main,
            "section" => TagName::Section,
            "search" => TagName::Search,
            "article" => TagName::Article,
            "aside" => TagName::Aside,
            "details" => TagName::Details,
            "dialog" => TagName::Dialog,
            "summary" => TagName::Summary,
            "data" => TagName::Data,

            // Meta Info
            "meta" => TagName::Meta,
            "base" => TagName::Base, 
            "basefont" => TagName::Basefont,

            // Programming
            "script" => TagName::Script,
            "noscript" => TagName::Noscript,
            "applet" => TagName::Applet,
            "embed" => TagName::Embed,
            "object" => TagName::Object,
            "param" => TagName::Param,

            "command" => TagName::Command,
            "keygen" => TagName::Keygen,
            
            _ => TagName::Other(s),
        }
    }
    pub fn x_from(s: SpecTag) -> TagName {
        TagName::X(s)
    }

    pub fn is_void(&self) -> bool {        
        // area, base, br, col, command, embed, hr, img, input, keygen, link, meta, param, source, track, wbr
        match self {
            TagName::Area |
            TagName::Base |
            TagName::Br |
            TagName::Col |
            TagName::Command |
            TagName::Embed |
            TagName::Hr |
            TagName::Img |
            TagName::Input |
            TagName::Keygen |
            TagName::Link |
            TagName::Meta |
            TagName::Param |
            TagName::Source |
            TagName::Track |
            TagName::Wbr => true,
            _ => false,
        }
    }
    
    /*fn breaker(&self) -> Breaker {
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

            //Styles and Semantics
            Span => ,
            Div => ,
            Style => ,           
            
            Header |
            Hgroup |
            Footer |
            Main |
            Section |
            Search |
            Article |
            Aside |
            Details |
            Dialog |
            Summary |
            Data => ,
            
            // Meta Info
            Head |
            Meta |
            Base |
            Basefont => ,
            
            // Programming
            TagName::Script |
            TagName::Noscript |
            TagName::Applet |
            TagName::Embed |
            TagName::Object |
            TagName::Param => Breaker::Paragraph,

            TagName::X(_) => Breaker::Word,
            TagName::Other(_name) => Breaker::Word,
        }
    }*/
}
