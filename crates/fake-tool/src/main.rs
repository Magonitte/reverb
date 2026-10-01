//! Simula ferramentas externas (yt-dlp, ffmpeg, servidor de PO token) nos testes.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::process::{self, Command};
use std::thread::sleep;
use std::time::Duration;

#[derive(Default)]
struct Opts {
    version: bool,
    sleep_s: Option<f64>,
    spawn_child_sleep_s: Option<f64>,
    stdout_lines: Option<String>,
    exit_code: Option<i32>,
    stderr: Option<String>,
    http_ping: Option<u16>,
}

fn parse(args: &[String]) -> Result<Opts, String> {
    let mut opts = Opts::default();
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        let mut value = |name: &str| {
            it.next()
                .cloned()
                .ok_or_else(|| format!("{name} exige um valor"))
        };
        match arg.as_str() {
            "--version" | "-version" => opts.version = true,
            "--sleep" => opts.sleep_s = Some(parse_num(&value("--sleep")?)?),
            "--spawn-child-sleep" => {
                opts.spawn_child_sleep_s = Some(parse_num(&value("--spawn-child-sleep")?)?)
            }
            "--stdout-lines" => opts.stdout_lines = Some(value("--stdout-lines")?),
            "--exit" => opts.exit_code = Some(parse_num::<i32>(&value("--exit")?)?),
            "--stderr" => opts.stderr = Some(value("--stderr")?),
            "--http-ping" => opts.http_ping = Some(parse_num::<u16>(&value("--http-ping")?)?),
            other => return Err(format!("argumento desconhecido: {other}")),
        }
    }
    Ok(opts)
}

fn parse_num<T: std::str::FromStr>(text: &str) -> Result<T, String> {
    text.parse()
        .map_err(|_| format!("valor numérico inválido: {text}"))
}

fn serve_ping(port: u16) -> ! {
    let listener = TcpListener::bind(("127.0.0.1", port)).unwrap_or_else(|e| {
        eprintln!("não foi possível abrir a porta {port}: {e}");
        process::exit(2);
    });
    for stream in listener.incoming().flatten() {
        let mut reader = BufReader::new(&stream);
        let mut request_line = String::new();
        if reader.read_line(&mut request_line).is_err() {
            continue;
        }
        // Descarta os cabeçalhos até a linha em branco.
        let mut header = String::new();
        while reader
            .read_line(&mut header)
            .map(|n| n > 2)
            .unwrap_or(false)
        {
            header.clear();
        }
        let (status, body) = if request_line.starts_with("GET /ping") {
            ("200 OK", r#"{"version":"fake"}"#)
        } else {
            ("404 Not Found", "{}")
        };
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let mut writer = &stream;
        let _ = writer.write_all(response.as_bytes());
        let _ = writer.flush();
    }
    process::exit(0);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let opts = match parse(&args) {
        Ok(opts) => opts,
        Err(message) => {
            eprintln!("{message}");
            process::exit(64);
        }
    };

    if let Some(text) = &opts.stderr {
        eprintln!("{text}");
    }

    if opts.version {
        let version = std::env::var("FAKE_TOOL_VERSION").unwrap_or_else(|_| "1.0.0".to_string());
        println!("{version}");
    }

    if let Some(path) = &opts.stdout_lines {
        let content = std::fs::read_to_string(path).unwrap_or_else(|e| {
            eprintln!("não foi possível ler {path}: {e}");
            process::exit(2);
        });
        let stdout = std::io::stdout();
        for line in content.lines() {
            let mut handle = stdout.lock();
            let _ = writeln!(handle, "{line}");
            let _ = handle.flush();
            drop(handle);
            sleep(Duration::from_millis(50));
        }
    }

    if let Some(port) = opts.http_ping {
        serve_ping(port);
    }

    if let Some(seconds) = opts.spawn_child_sleep_s {
        let exe = std::env::current_exe().expect("current_exe");
        let mut child = Command::new(exe)
            .args(["--sleep", &seconds.to_string()])
            .spawn()
            .expect("falha ao criar o processo filho");
        println!("CHILD_PID={}", child.id());
        let _ = std::io::stdout().flush();
        sleep(Duration::from_secs_f64(seconds));
        let _ = child.wait();
    }

    if let Some(seconds) = opts.sleep_s {
        sleep(Duration::from_secs_f64(seconds));
    }

    process::exit(opts.exit_code.unwrap_or(0));
}
