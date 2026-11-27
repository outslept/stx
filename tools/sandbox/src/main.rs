use std::borrow::Cow;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Instant;

use oxc_css_parser::ast::{Declaration, QualifiedRule, Stylesheet};
use oxc_css_parser::error::Error as ParserError;
use oxc_css_parser::{
    Allocator, Parse, Parser, ParserBuilder, ParserOptions, Span, Syntax, TemplatePlaceholder,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ParseRequest {
    source: String,
    #[serde(default)]
    syntax: RequestSyntax,
    #[serde(default)]
    mode: ParseMode,
    #[serde(default = "default_true")]
    collect_comments: bool,
    #[serde(default)]
    block_contents: bool,
    #[serde(default)]
    template_placeholder: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum RequestSyntax {
    #[default]
    Css,
    Scss,
    Sass,
    Less,
}

impl From<RequestSyntax> for Syntax {
    fn from(value: RequestSyntax) -> Self {
        match value {
            RequestSyntax::Css => Self::Css,
            RequestSyntax::Scss => Self::Scss,
            RequestSyntax::Sass => Self::Sass,
            RequestSyntax::Less => Self::Less,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ParseMode {
    #[default]
    Stylesheet,
    QualifiedRule,
    Declaration,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ErrorDto {
    message: String,
    kind: serde_json::Value,
    span: Span,
}

impl ErrorDto {
    fn from_parser_error(err: &ParserError) -> Self {
        Self {
            message: err.kind.to_string(),
            kind: serde_json::to_value(&err.kind).unwrap_or(serde_json::Value::Null),
            span: err.span,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ParseResponse {
    ok: bool,
    parse_time_us: u128,
    ast: Option<serde_json::Value>,
    comments: serde_json::Value,
    fatal_error: Option<ErrorDto>,
    recoverable_errors: Vec<ErrorDto>,
    warnings: Vec<String>,
}

fn run_parse<'a, T>(parser: &mut Parser<'a>) -> (Option<serde_json::Value>, Option<ErrorDto>)
where
    T: Parse<'a> + Serialize,
{
    match parser.parse::<T>() {
        Ok(node) => match serde_json::to_value(&node) {
            Ok(val) => (Some(val), None),
            Err(err) => (
                None,
                Some(ErrorDto {
                    message: format!("AST serialization failed: {err}"),
                    kind: serde_json::Value::Null,
                    span: Span { start: 0, end: 0 },
                }),
            ),
        },
        Err(err) => (None, Some(ErrorDto::from_parser_error(&err))),
    }
}

fn handle_parse(body: &[u8]) -> Result<ParseResponse, String> {
    let req: ParseRequest =
        serde_json::from_slice(body).map_err(|e| format!("Invalid JSON body: {e}"))?;

    let syntax: Syntax = req.syntax.into();
    let mut warnings = Vec::new();

    let template_placeholder = if req.template_placeholder {
        if syntax == Syntax::Scss {
            Some(TemplatePlaceholder {
                prefix: "PLACEHOLDER_",
            })
        } else {
            warnings.push(
                "template_placeholder is only supported with SCSS syntax (ignored here)."
                    .to_string(),
            );
            None
        }
    } else {
        None
    };

    let options = ParserOptions {
        template_placeholder,
        block_contents: req.block_contents,
    };

    let allocator = Allocator::default();
    let mut builder = ParserBuilder::new(&allocator, &req.source)
        .syntax(syntax)
        .options(options);

    if req.collect_comments {
        builder = builder.comments();
    }

    let start = Instant::now();
    let mut parser = builder.build();

    let (ast, fatal_error) = match req.mode {
        ParseMode::Stylesheet => run_parse::<Stylesheet>(&mut parser),
        ParseMode::QualifiedRule => run_parse::<QualifiedRule>(&mut parser),
        ParseMode::Declaration => run_parse::<Declaration>(&mut parser),
    };
    let elapsed_us = start.elapsed().as_micros();

    let comments =
        serde_json::to_value(parser.comments()).unwrap_or(serde_json::Value::Array(vec![]));
    let recoverable_errors = parser
        .recoverable_errors()
        .iter()
        .map(ErrorDto::from_parser_error)
        .collect();

    Ok(ParseResponse {
        ok: fatal_error.is_none(),
        parse_time_us: elapsed_us,
        ast,
        comments,
        fatal_error,
        recoverable_errors,
        warnings,
    })
}

fn load_html() -> Cow<'static, str> {
    let local_path = concat!(env!("CARGO_MANIFEST_DIR"), "/sandbox.html");
    std::fs::read_to_string(local_path)
        .map(Cow::Owned)
        .unwrap_or_else(|_| Cow::Borrowed(include_str!("../sandbox.html")))
}

fn write_response(
    stream: &mut TcpStream,
    status: &str,
    content_type: &str,
    body: &[u8],
) -> std::io::Result<()> {
    let header = format!(
        "HTTP/1.1 {status}\r\n\
         Content-Type: {content_type}\r\n\
         Content-Length: {}\r\n\
         Cache-Control: no-store\r\n\
         Connection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(header.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()
}

fn handle_client(mut stream: TcpStream) {
    let Ok(cloned) = stream.try_clone() else {
        return;
    };
    let mut reader = BufReader::new(cloned);

    let mut request_line = String::new();
    if reader.read_line(&mut request_line).unwrap_or(0) == 0 {
        return;
    }

    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("/");

    let mut content_length: usize = 0;
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) | Err(_) => return,
            Ok(_) => {
                let trimmed = line.trim_end_matches(['\r', '\n']);
                if trimmed.is_empty() {
                    break;
                }
                if let Some((name, value)) = trimmed.split_once(':')
                    && name.trim().eq_ignore_ascii_case("content-length")
                {
                    content_length = value.trim().parse().unwrap_or(0);
                }
            }
        }
    }

    if content_length > 16 * 1024 * 1024 {
        let _ = write_response(
            &mut stream,
            "413 Payload Too Large",
            "text/plain; charset=utf-8",
            b"Payload too large",
        );
        return;
    }

    let mut body = vec![0u8; content_length];
    if content_length > 0 && reader.read_exact(&mut body).is_err() {
        let _ = write_response(
            &mut stream,
            "400 Bad Request",
            "text/plain; charset=utf-8",
            b"Failed to read request body",
        );
        return;
    }

    match (method, path) {
        ("GET", "/") | ("GET", "/index.html") => {
            let html = load_html();
            let _ = write_response(
                &mut stream,
                "200 OK",
                "text/html; charset=utf-8",
                html.as_bytes(),
            );
        }
        ("POST", "/api/parse") => match handle_parse(&body) {
            Ok(resp) => {
                let json = serde_json::to_vec(&resp).unwrap_or_else(|e| {
                    format!(r#"{{"ok":false,"fatalError":{{"message":"{e}"}}}}"#).into_bytes()
                });
                let _ = write_response(
                    &mut stream,
                    "200 OK",
                    "application/json; charset=utf-8",
                    &json,
                );
            }
            Err(err) => {
                let json = serde_json::json!({ "error": err }).to_string();
                let _ = write_response(
                    &mut stream,
                    "400 Bad Request",
                    "application/json; charset=utf-8",
                    json.as_bytes(),
                );
            }
        },
        ("GET", "/health") => {
            let _ = write_response(&mut stream, "200 OK", "text/plain; charset=utf-8", b"ok");
        }
        _ => {
            let _ = write_response(
                &mut stream,
                "404 Not Found",
                "text/plain; charset=utf-8",
                b"Not Found",
            );
        }
    }
}

fn main() {
    let port = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(4173);
    let addr = format!("127.0.0.1:{port}");
    let listener = TcpListener::bind(&addr).expect("failed to bind TCP listener");

    eprintln!("oxc-css-parser sandbox listening on http://{addr}");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(move || handle_client(stream));
            }
            Err(err) => eprintln!("accept error: {err}"),
        }
    }
}
