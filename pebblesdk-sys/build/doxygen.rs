use std::ops::Deref;
use std::str::Chars;

use bindgen::callbacks::ParseCallbacks;

#[derive(Debug)]
pub struct ParseDoxygen;

impl ParseCallbacks for ParseDoxygen {
    fn process_comment(&self, comment: &str) -> Option<String> {
        Doc::new(comment).render()
    }
}

#[derive(Clone, Debug)]
struct Tokens<'a> {
    chars: Chars<'a>,
    peeked: (&'a str, Option<char>),
    start_of_line: bool,
}

impl<'a> Tokens<'a> {
    pub(self) fn new(input: &'a str) -> Tokens<'a> {
        Tokens {
            chars: input.chars(),
            peeked: (input, None),
            start_of_line: true,
        }
    }

    fn as_str(&self) -> &'a str {
        if self.peeked.1.is_some() {
            self.peeked.0
        } else {
            self.chars.as_str()
        }
    }

    fn take_range(&self, start: &'a str) -> &'a str {
        start.split_at(start.len() - self.as_str().len()).0
    }

    pub(self) fn peek(&self) -> Option<Token<'a>> {
        let mut future_tokens = self.clone();
        let token = future_tokens.next()?;
        Some(Token {
            token,
            future_tokens,
        })
    }

    pub(self) fn consume(&mut self, token: Token<'a>) -> &'a str {
        self.chars = token.future_tokens.chars;
        self.peeked = token.future_tokens.peeked;
        self.start_of_line = token.future_tokens.start_of_line;
        token.token
    }
}

impl Tokens<'_> {
    fn next_char(&mut self) -> Option<char> {
        match self.peeked.1 {
            Some(char) => {
                self.peeked.1 = None;
                Some(char)
            }
            None => self.chars.next(),
        }
    }

    fn peek_char(&mut self) -> Option<char> {
        match self.peeked.1 {
            Some(char) => Some(char),
            None => {
                self.peeked = (self.chars.as_str(), self.chars.next());
                self.peeked.1
            }
        }
    }

    fn maybe_char(&mut self, c: char) -> bool {
        if self.peek_char() == Some(c) {
            self.next_char();
            return true;
        }
        false
    }
}

impl<'a> Iterator for Tokens<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start_of_line {
            let prefixed = self.maybe_char('!');
            let _ = (self.maybe_char('<') || prefixed) && self.maybe_char(' ');
            self.start_of_line = false;
        }

        let start = self.as_str();
        match self.next_char()? {
            '\n' => {
                self.start_of_line = true;
                Some(self.take_range(start))
            }
            _ => {
                let mut is_whitespace = true;
                while let Some(c) = self.peek_char() {
                    if is_whitespace {
                        if matches!(c, ' ' | '\t' | '\u{A0}') {
                            self.next_char();
                            continue;
                        }
                        is_whitespace = false;
                    }
                    if !matches!(c, '0'..='9' | 'A'..='Z' | '_' | 'a'..='z') {
                        break;
                    }
                    self.next_char();
                }
                Some(self.take_range(start))
            }
        }
    }
}

#[derive(Debug)]
struct Token<'a> {
    token: &'a str,
    future_tokens: Tokens<'a>,
}

impl<'a> Token<'a> {
    pub(self) fn peek(mut self) -> Option<Token<'a>> {
        self.token = self.future_tokens.next()?;
        Some(self)
    }
}

impl<'a> Deref for Token<'a> {
    type Target = &'a str;

    fn deref(&self) -> &Self::Target {
        &self.token
    }
}

#[derive(Debug)]
enum Command<'a> {
    Brief,
    Code { language: &'a str },
    Deprecated,
    Invalidate,
    Plain,
    Returns,
    SeeAlso,
    Skip,
}

fn code_language<'a>(opening_token: Token<'a>) -> Option<(&'a str, Token<'a>)> {
    if *opening_token == "{" {
        let extension_token = opening_token.peek()?;
        let language = extension_token.strip_prefix(".")?;
        let closing_token = extension_token.peek()?;
        if *closing_token == "}" {
            return Some((language, closing_token));
        }
    }
    None
}

fn block_type<'a>(command_token: Token<'a>) -> Option<(Command<'a>, Token<'a>)> {
    let command = match command_token
        .strip_prefix("@")
        .or_else(|| command_token.strip_prefix("\\"))?
    {
        "brief" => Command::Brief,
        "code" => {
            let (language, end_token) = code_language(command_token.peek()?)?;
            return Some((Command::Code { language }, end_token));
        }
        "deprecated" => Command::Deprecated,
        "" => match command_token.peek()? {
            new_block if *new_block == "{" => {
                return Some((Command::Invalidate, new_block));
            }
            _ => return None,
        },
        "note" => Command::Plain,
        "return" => Command::Returns,
        "see" | "sa" => Command::SeeAlso,
        "param" | "endcode" | "addtogroup" | "internal" => Command::Skip,
        _ => return None,
    };
    Some((command, command_token))
}

#[derive(Debug)]
pub struct Doc<'a> {
    tokens: Tokens<'a>,
    deprecated: Option<String>,
    brief: Option<String>,
    message: Vec<String>,
    returns: Option<String>,
    see_also: Vec<String>,
}

fn inline_command<'a>(command_token: Token<'a>) -> Option<(Vec<&'a str>, Token<'a>)> {
    let (left, right) = match *command_token {
        "\\a" => Some(("_", "_")),
        "\\b" => Some(("**", "**")),
        "\\ref" => Some(("[`", "`]")),
        _ => None,
    }?;
    let arg_token = command_token.peek()?;
    let (whitespace, arg) = arg_token.split_at(arg_token.len() - arg_token.trim_start().len());
    Some((vec![whitespace, left, arg, right], arg_token))
}

impl<'a> Doc<'a> {
    pub(crate) fn new(comment: &'a str) -> Doc<'a> {
        Doc {
            tokens: Tokens::new(comment),
            deprecated: None,
            brief: None,
            message: Vec::new(),
            returns: None,
            see_also: Vec::new(),
        }
    }

    fn body_token(&mut self, next_line: bool) -> Option<(Vec<&'a str>, bool)> {
        let next_token = self.tokens.peek()?;
        match next_token
            .chars()
            .next()
            .expect("tokens have non-zero length")
        {
            '\n' if next_line => loop {
                match self.tokens.peek() {
                    Some(token) if *token == "\n" => self.tokens.consume(token),
                    _ => break None,
                };
            },
            '@' | '\\' if next_line && block_type(self.tokens.peek()?).is_some() => None,
            c => {
                if c == '\\' {
                    if let Some((token_batch, end_token)) = inline_command(next_token) {
                        self.tokens.consume(end_token);
                        return Some((token_batch, false));
                    }
                }
                Some((
                    vec![self
                        .tokens
                        .next()
                        .expect("token should be avaliable if one was just peeked")],
                    c == '\n',
                ))
            }
        }
    }

    fn block_body(&mut self) -> Vec<&'a str> {
        let mut body = Vec::new();
        let mut next_line = false;
        while let Some((token_batch, is_newline)) = self.body_token(next_line) {
            body.extend_from_slice(&token_batch);
            next_line = is_newline;
        }

        if let Some(last) = body.iter().rposition(|token| token.trim() != "") {
            let _ = body.split_off(last + 1);
            body[0] = body[0].trim_start();
            body[last] = body[last].trim_end();
        }

        body
    }

    fn block_body_paragraph(&mut self) -> Vec<&'a str> {
        let mut body = self.block_body();
        if !body.is_empty() {
            if body[body.len() - 1]
                .chars()
                .next_back()
                .unwrap()
                .is_alphanumeric()
            {
                body.push(".");
            }
        }
        body
    }
}

impl Doc<'_> {
    fn code(&mut self, language: &str) {
        let body = self.block_body().join("");
        self.message.push(format!("```{}\n{}\n```", language, body));
    }

    fn brief(&mut self) {
        let body = self.block_body_paragraph();
        if !body.is_empty() {
            self.brief = Some(body.join(""));
        };
    }

    fn returns(&mut self) {
        let mut body = self.block_body_paragraph();
        if !body.is_empty() {
            let first_word;
            let mut chars = body[0].chars();
            if let Some(first_char) = chars.next() {
                if !first_char.is_lowercase() {
                    first_word = first_char.to_lowercase().to_string() + chars.as_str();
                    body[0] = &first_word;
                }
            }
            self.returns = Some(body.join(""));
        }
    }

    fn see_also(&mut self) {
        let body = self.block_body();
        if !body.is_empty() {
            self.see_also.push(body.join(""));
        };
    }

    fn deprecated(&mut self) {
        self.deprecated = Some(self.block_body_paragraph().join(""));
    }

    fn note(&mut self) {
        let mut body = self.block_body_paragraph();
        if !body.is_empty() {
            let first_word;
            let mut chars = body[0].chars();
            if let Some(first_char) = chars.next() {
                if !first_char.is_uppercase() {
                    first_word = first_char.to_uppercase().to_string() + chars.as_str();
                    body[0] = &first_word;
                }
            }
            self.message.push(body.join(""));
        };
    }

    fn reset(&mut self) {
        self.block_body();
        self.deprecated = None;
        self.brief = None;
        self.message = Vec::new();
        self.returns = None;
        self.see_also = Vec::new();
    }

    fn block(&mut self) -> bool {
        if let Some((command, end_token)) = self.tokens.peek().and_then(|token| block_type(token)) {
            self.tokens.consume(end_token);
            match command {
                Command::Brief => self.brief(),
                Command::Code { language } => self.code(language),
                Command::Deprecated => self.deprecated(),
                Command::Invalidate => self.reset(),
                Command::Plain => self.note(),
                Command::Returns => self.returns(),
                Command::SeeAlso => self.see_also(),
                Command::Skip => {
                    // consume the body but otherwise do nothing
                    self.block_body();
                }
            };
            true
        } else {
            false
        }
    }

    pub(crate) fn render(mut self) -> Option<String> {
        loop {
            if !self.block() {
                let body = self.block_body();
                if body.is_empty() {
                    break;
                } else {
                    self.message.push(body.join(""));
                };
            }
        }

        let mut output = Vec::new();
        if let Some(deprecated) = self.deprecated {
            output.push(format!("**Deprecated: {}**", deprecated))
        }
        if let Some(brief) = self.brief {
            output.push(brief + ".");
        }
        match (self.message.is_empty(), self.returns) {
            (false, Some(returns)) => {
                output.extend_from_slice(&self.message);
                let final_line = output.len() - 1;
                output[final_line] += &format!(" Returns {}", returns);
            }
            (false, None) => output.extend_from_slice(&self.message),
            (true, Some(returns)) => output.push(format!("Returns {}", returns)),
            _ => {}
        }
        if !self.see_also.is_empty() {
            let also = self.see_also.join(", ");
            output.push(format!("See also {}.", also));
        }
        Some(output.join("\n\n"))
    }
}
