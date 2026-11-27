//! Spike T012 / quickstart M0 dòng 0.10: đo giao thức model (R-6, contracts/model-protocol.md).
//!
//! Đo: tương thích OpenAI (`POST {base}/chat/completions`), `response_format: json_schema`, vị trí
//! phần suy nghĩ (`<think>` hay trường `reasoning_content`/`reasoning`), p50/p95 độ trễ, giới hạn ngữ
//! cảnh (2k..32k ký tự), và n câu trả lời JSON một dòng hợp lệ (đạt khi >= 95% n, mặc định 19/20).
//! TLS dùng kho chứng chỉ Windows (rustls-platform-verifier); proxy: `--proxy` hoặc biến môi trường
//! (`ALL_PROXY`/`HTTPS_PROXY`/`HTTP_PROXY`). Không có giá trị mạng nào trong code. Chỉ gửi nội dung
//! tổng hợp cố định, không có dữ liệu thật. Khoá API chỉ đọc từ `TEACH_API_KEY`, không bao giờ in.
//!
//! Chạy trên MÁY CÔNG TY, trong mạng công ty (PowerShell), lần lượt với DeepSeek rồi GLM:
//! ```powershell
//! $env:TEACH_API_KEY = "<key>"
//! cargo run --example spike_probe -- --url https://<host>/v1 --model <name> [--proxy http://<proxy>:<port>] [--n 20] [--timeout 30]
//! Remove-Item Env:TEACH_API_KEY
//! ```
//! Kết quả (dòng `0.10: ...` và các dòng nhãn) PHẢI ghi vào specs/001-teach-a-task/research.md
//! §Kết quả M0. `--insecure-http-loopback-only` cho phép `http://127.0.0.1|localhost` (máy chủ giả).

use serde_json::{Value, json};
use std::process::{Command, ExitCode};
use std::time::{Duration, Instant};
use ureq::config::Config;
use ureq::tls::{RootCerts, TlsConfig};
use ureq::{Agent, Proxy};

const SYSTEM: &str = "You pick one UI element for a step. Answer with one line of JSON only.";
const HEADER: &str = "STEP: Click the Save button on the toolbar\nACTION: click\n\
WINDOW: \"Invoice - Excel\" (EXCEL.EXE)\nRECORDED TARGET: role=Button name=\"Save\" automation_id=FileSave\nCANDIDATES:\n";
const CANDS: [&str; 3] = [
    "1. Button \"Save as copy\" id=FileSave2 [enabled]",
    "2. Button \"Save As...\" id=FileSaveAs [enabled]",
    "3. MenuItem \"Save\" [enabled]",
];
const TAIL: &str = "\nAnswer with one line of JSON: {\"choice\": <number>} or {\"choice\": \"none\"}";

struct Args {
    url: String,
    model: String,
    proxy: Option<String>,
    n: usize,
    timeout: u64,
    loopback_http: bool,
}

struct Reply {
    status: u16,
    body: String,
    ms: u128,
}

struct Ctx {
    agent: Agent,
    base: String,
    model: String,
    key: Option<String>,
}

fn parse_args() -> Result<Args, String> {
    let mut a = Args { url: String::new(), model: String::new(), proxy: None, n: 20, timeout: 30, loopback_http: false };
    let mut it = std::env::args().skip(1);
    while let Some(f) = it.next() {
        let mut val = |name: &str| it.next().ok_or(format!("{name} needs a value"));
        match f.as_str() {
            "--url" => a.url = val("--url")?.trim_end_matches('/').to_string(),
            "--model" => a.model = val("--model")?,
            "--proxy" => a.proxy = Some(val("--proxy")?),
            "--n" => a.n = val("--n")?.parse().map_err(|_| "--n: not a number")?,
            "--timeout" => a.timeout = val("--timeout")?.parse().map_err(|_| "--timeout: not a number")?,
            "--insecure-http-loopback-only" => a.loopback_http = true,
            other => return Err(format!("unknown argument {other}")),
        }
    }
    if a.url.is_empty() || a.model.is_empty() || a.n == 0 {
        return Err("usage: spike_probe --url <base_url> --model <name> [--proxy <url>] [--n 20] [--timeout 30] [--insecure-http-loopback-only]".into());
    }
    Ok(a)
}

/// Host of an http(s) URL (no userinfo, no port); None if scheme is neither.
fn scheme_host(url: &str) -> Option<(&str, String)> {
    let (scheme, rest) = url.split_once("://")?;
    let auth = rest.split(['/', '?', '#']).next()?;
    if auth.contains('@') {
        return None; // no credentials in URL
    }
    let host = auth.rsplit_once(':').map_or(auth, |(h, p)| if p.chars().all(|c| c.is_ascii_digit()) { h } else { auth });
    Some((scheme, host.to_ascii_lowercase()))
}

/// Remove `user:pass@` from a proxy URL before printing.
fn strip_userinfo(u: &str) -> String {
    match u.split_once("://") {
        Some((s, r)) => format!("{s}://{}", r.rsplit_once('@').map_or(r, |(_, h)| h)),
        None => u.rsplit_once('@').map_or(u, |(_, h)| h).to_string(),
    }
}

fn mask(ctx_key: &Option<String>, s: &str) -> String {
    match ctx_key {
        Some(k) if !k.is_empty() => s.replace(k.as_str(), "***"),
        _ => s.to_string(),
    }
}

/// Error class label for a transport failure (no key echoed).
fn classify(e: &ureq::Error) -> &'static str {
    use ureq::Error as E;
    let d = format!("{e:?}");
    match e {
        E::Timeout(_) => "timeout",
        E::InvalidProxyUrl => "proxy_error",
        E::HostNotFound => "dns_error",
        E::Rustls(_) | E::Tls(_) => "tls_error",
        _ if d.contains("Certificate") || d.contains("certificate") || d.contains("Rustls") || d.contains("Unknown") && d.contains("Issuer") => "tls_error",
        _ if d.to_lowercase().contains("proxy") => "proxy_error",
        _ => "network_error",
    }
}

fn status_class(s: u16) -> &'static str {
    match s {
        200..=299 => "ok",
        401 | 403 => "auth_error",
        404 => "not_found (model or path)",
        429 => "rate_limited",
        500..=599 => "server_error",
        _ => "http_error",
    }
}

impl Ctx {
    fn send(&self, path: &str, body: Option<&Value>) -> Result<Reply, String> {
        let url = format!("{}{}", self.base, path);
        let t = Instant::now();
        let res = if let Some(b) = body {
            let mut r = self.agent.post(&url).header("Content-Type", "application/json");
            if let Some(k) = &self.key {
                r = r.header("Authorization", format!("Bearer {k}"));
            }
            r.send(b.to_string())
        } else {
            let mut r = self.agent.get(&url);
            if let Some(k) = &self.key {
                r = r.header("Authorization", format!("Bearer {k}"));
            }
            r.call()
        };
        match res {
            Ok(mut resp) => {
                let status = resp.status().as_u16();
                let body = resp.body_mut().read_to_string().map_err(|e| format!("{}: {}", classify(&e), mask(&self.key, &e.to_string())))?;
                Ok(Reply { status, body, ms: t.elapsed().as_millis() })
            }
            Err(e) => Err(format!("{}: {}", classify(&e), mask(&self.key, &e.to_string()))),
        }
    }

    fn chat(&self, user: &str, extra: Option<(&str, Value)>) -> Result<Reply, String> {
        let mut b = json!({"model": self.model, "temperature": 0, "stream": false,
            "messages": [{"role":"system","content":SYSTEM},{"role":"user","content":user}]});
        if let Some((k, v)) = extra {
            b[k] = v;
        }
        self.send("/chat/completions", Some(&b))
    }

    fn fail_line(&self, r: &Reply) -> String {
        let snip: String = mask(&self.key, &r.body).chars().take(200).collect();
        format!("HTTP {} {} body={:?}", r.status, status_class(r.status), snip)
    }
}

fn question() -> String {
    format!("{HEADER}{}{TAIL}", CANDS.join("\n"))
}

fn message(body: &str) -> Option<Value> {
    serde_json::from_str::<Value>(body).ok()?.pointer("/choices/0/message").cloned()
}

fn strip_think(s: &str) -> String {
    let mut out = s.to_string();
    for tag in ["think", "thinking"] {
        let (o, c) = (format!("<{tag}>"), format!("</{tag}>"));
        while let Some(i) = out.find(&o) {
            match out[i..].find(&c) {
                Some(j) => out.replace_range(i..i + j + c.len(), ""),
                None => out.truncate(i),
            }
        }
    }
    out
}

/// First brace-balanced `{...}` (braces inside strings ignored).
fn first_json_object(s: &str) -> Option<&str> {
    let start = s.find('{')?;
    let (mut depth, mut in_str, mut esc) = (0i32, false, false);
    for (i, c) in s[start..].char_indices() {
        if in_str {
            match (esc, c) {
                (true, _) => esc = false,
                (_, '\\') => esc = true,
                (_, '"') => in_str = false,
                _ => {}
            }
        } else if c == '"' {
            in_str = true;
        } else if c == '{' {
            depth += 1;
        } else if c == '}' {
            depth -= 1;
            if depth == 0 {
                return Some(&s[start..start + i + 1]);
            }
        }
    }
    None
}

/// Valid = single-line content (after think removal) with integer `choice` in 1..=3.
fn valid_choice(content: &str) -> Result<(), &'static str> {
    let c = strip_think(content);
    let c = c.trim();
    let obj = first_json_object(c).ok_or("no_json")?;
    let v: Value = serde_json::from_str(obj).map_err(|_| "bad_json")?;
    let n = v.get("choice").and_then(Value::as_i64).ok_or("out_of_range")?;
    if !(1..=CANDS.len() as i64).contains(&n) {
        return Err("out_of_range");
    }
    if c.contains('\n') {
        return Err("not_single_line");
    }
    Ok(())
}

fn pct(sorted: &[u128], p: f64) -> u128 {
    if sorted.is_empty() {
        return 0;
    }
    let rank = ((p / 100.0) * sorted.len() as f64).ceil() as usize;
    sorted[rank.clamp(1, sorted.len()) - 1]
}

fn windows_proxy_info() {
    let key = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings";
    for name in ["ProxyEnable", "ProxyServer", "AutoConfigURL"] {
        let out = Command::new("reg").args(["query", key, "/v", name]).output();
        let line = match out {
            Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
                .lines()
                .find(|l| l.contains(name))
                .map(|l| l.split_whitespace().skip(2).collect::<Vec<_>>().join(" "))
                .unwrap_or_default(),
            _ => "(not set / unreadable)".into(),
        };
        println!("windows_proxy.{name}: {}", strip_userinfo(&line));
    }
}

fn main() -> ExitCode {
    let a = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    match scheme_host(&a.url) {
        Some(("https", _)) => {}
        Some(("http", h)) if a.loopback_http && (h == "127.0.0.1" || h == "localhost") => {}
        _ => {
            eprintln!("refused: base_url must be https:// without credentials (http only for 127.0.0.1/localhost with --insecure-http-loopback-only)");
            return ExitCode::from(2);
        }
    }
    let key = std::env::var("TEACH_API_KEY").ok().filter(|k| !k.is_empty());
    println!("api_key: {}", if key.is_some() { "TEACH_API_KEY set (hidden)" } else { "TEACH_API_KEY not set, no Authorization header" });
    println!("url: {}  model: {}  n: {}  timeout: {}s", a.url, a.model, a.n, a.timeout);

    windows_proxy_info();
    let proxy = match &a.proxy {
        Some(p) => match Proxy::new(p) {
            Ok(p) => Some(p),
            Err(_) => {
                eprintln!("--proxy: invalid proxy URL");
                return ExitCode::from(2);
            }
        },
        None => Proxy::try_from_env(),
    };
    match (&a.proxy, &proxy) {
        (Some(p), _) => println!("proxy: --proxy {}", strip_userinfo(p)),
        (None, Some(p)) => println!("proxy: env (HTTPS_PROXY/ALL_PROXY...) {}", strip_userinfo(&p.uri().to_string())),
        (None, None) => println!("proxy: none (direct)"),
    }
    let cfg = Config::builder()
        .tls_config(TlsConfig::builder().root_certs(RootCerts::PlatformVerifier).build())
        .proxy(proxy)
        .http_status_as_error(false)
        .timeout_global(Some(Duration::from_secs(a.timeout)))
        .build();
    let ctx = Ctx { agent: Agent::new_with_config(cfg), base: a.url.clone(), model: a.model.clone(), key };

    // 1. OpenAI compat
    let mut openai = false;
    match ctx.chat("Reply with the single word: ok", None) {
        Ok(r) if (200..300).contains(&r.status) => {
            openai = message(&r.body).is_some_and(|m| m.get("content").is_some_and(Value::is_string));
            println!("1 openai_compat: {} (HTTP {}, {} ms, choices[0].message.content {})", if openai { "yes" } else { "no" }, r.status, r.ms, if openai { "present" } else { "missing" });
        }
        Ok(r) => println!("1 openai_compat: no — {}", ctx.fail_line(&r)),
        Err(e) => println!("1 openai_compat: no — {e}"),
    }
    match ctx.send("/models", None) {
        Ok(r) => println!("1 models (info): HTTP {} {}", r.status, status_class(r.status)),
        Err(e) => println!("1 models (info): {e}"),
    }

    // 2 + 3. n questions, thinking location
    let q = question();
    let (mut ok, mut lat, mut tag, mut field) = (0usize, Vec::<u128>::new(), false, false);
    for i in 1..=a.n {
        match ctx.chat(&q, None) {
            Ok(r) if (200..300).contains(&r.status) => {
                lat.push(r.ms);
                let m = message(&r.body);
                let content = m.as_ref().and_then(|m| m["content"].as_str()).unwrap_or("");
                tag |= content.contains("<think>") || content.contains("<thinking>");
                field |= m.as_ref().is_some_and(|m| ["reasoning_content", "reasoning"].iter().any(|k| m.get(k).is_some_and(|v| !v.is_null() && v.as_str() != Some(""))));
                match valid_choice(content) {
                    Ok(()) => ok += 1,
                    Err(why) => println!("2 q{i:02}: INVALID ({why}) {} ms content={:?}", r.ms, content.chars().take(120).collect::<String>()),
                }
            }
            Ok(r) => println!("2 q{i:02}: {}", ctx.fail_line(&r)),
            Err(e) => println!("2 q{i:02}: {e}"),
        }
    }
    lat.sort_unstable();
    let (p50, p95) = (pct(&lat, 50.0), pct(&lat, 95.0));
    println!("2 json_one_line: {ok}/{} valid; latency over {} answered: p50={p50} ms p95={p95} ms", a.n, lat.len());
    let thinking = if tag { "think_tag" } else if field { "reasoning_field" } else { "none" };
    println!("3 thinking: {thinking}");

    // 4. json_schema
    let rf = json!({"type":"json_schema","json_schema":{"name":"pick","strict":true,"schema":{
        "type":"object","properties":{"choice":{"type":"integer","minimum":1,"maximum":3}},
        "required":["choice"],"additionalProperties":false}}});
    let schema = match ctx.chat(&q, Some(("response_format", rf))) {
        Ok(r) if (200..300).contains(&r.status) => {
            let content = message(&r.body).and_then(|m| m["content"].as_str().map(str::to_string)).unwrap_or_default();
            let v = valid_choice(&content).is_ok();
            println!("4 json_schema: {} (HTTP {})", if v { "yes" } else { "no (accepted but answer invalid)" }, r.status);
            if v { "yes".to_string() } else { "no".to_string() }
        }
        Ok(r) => {
            println!("4 json_schema: error — {}", ctx.fail_line(&r));
            format!("error({})", r.status)
        }
        Err(e) => {
            println!("4 json_schema: error — {e}");
            "error".to_string()
        }
    };

    // 5. context limit
    let mut ctx_ok = 0usize;
    for size in [2_000usize, 4_000, 8_000, 16_000, 32_000] {
        let mut filler = String::new();
        let mut i = 4;
        while filler.len() < size {
            filler.push_str(&format!("{i}. Button \"Filler item {i}\" id=Fill{i} [enabled]\n"));
            i += 1;
        }
        let user = format!("{HEADER}{}\n{}{TAIL}", CANDS.join("\n"), filler.trim_end());
        match ctx.chat(&user, None) {
            Ok(r) if (200..300).contains(&r.status) => {
                println!("5 ctx {size} chars: ok ({} ms)", r.ms);
                ctx_ok = size;
            }
            Ok(r) => {
                println!("5 ctx {size} chars: FAIL {}", ctx.fail_line(&r));
                break;
            }
            Err(e) => {
                println!("5 ctx {size} chars: FAIL {e}");
                break;
            }
        }
    }

    let pass = ok * 100 >= a.n * 95;
    println!(
        "0.10: openai={} json_schema={} thinking={thinking} p50_ms={p50} p95_ms={p95} ctx={ctx_ok} json_ok={ok}/{} {}",
        if openai { "yes" } else { "no" },
        schema,
        a.n,
        if pass { "PASS" } else { "FAIL" }
    );
    if pass { ExitCode::SUCCESS } else { ExitCode::from(1) }
}
