use std::io::{IsTerminal, Write, stdout};
use std::sync::Mutex;
use std::time::Duration;

static STATUS: Mutex<Option<String>> = Mutex::new(None);

fn interactive() -> bool {
    stdout().is_terminal()
}

fn write_line(prefix: &str, message: &str) {
    let status = STATUS.lock().unwrap_or_else(|e| e.into_inner());
    let mut out = stdout().lock();

    if interactive() {
        let _ = write!(out, "\r\x1b[2K");
    }

    if interactive() {
        let _ = writeln!(out, "{prefix}{message}\x1b[0m");
    } else {
        let _ = writeln!(out, "{message}");
    }

    if interactive()
        && let Some(status) = status.as_deref()
    {
        let _ = write!(out, "\x1b[36m{status}\x1b[0m");
    }

    let _ = out.flush();
}

pub fn info(message: impl AsRef<str>) {
    write_line("", message.as_ref());
}

pub fn ok(message: impl AsRef<str>) {
    write_line("\x1b[32m", message.as_ref());
}

pub fn warn(message: impl AsRef<str>) {
    write_line("\x1b[33m", message.as_ref());
}

pub fn error(message: impl AsRef<str>) {
    write_line("\x1b[31m", message.as_ref());
}

pub fn status(text: Option<String>) {
    let mut status = STATUS.lock().unwrap_or_else(|e| e.into_inner());
    *status = text;

    if interactive() {
        let mut out = stdout().lock();
        let _ = write!(out, "\r\x1b[2K");

        if let Some(status) = status.as_deref() {
            let _ = write!(out, "\x1b[36m{status}\x1b[0m");
        }

        let _ = out.flush();
    }
}

pub fn ask(question: &str) {
    status(None);
    print!("\x1b[96m{question}\x1b[0m");
    let _ = stdout().flush();
}

pub fn prompt(question: &str) -> std::io::Result<String> {
    ask(question);

    let mut line = String::new();

    if std::io::stdin().read_line(&mut line)? == 0 {
        return Err(std::io::ErrorKind::UnexpectedEof.into());
    }

    Ok(line.trim().to_owned())
}

pub fn prompt_password(question: &str) -> std::io::Result<String> {
    status(None);

    rpassword::prompt_password(format!("\x1b[96m{question}\x1b[0m"))
}

pub fn duration(d: Duration) -> String {
    let s = d.as_secs();

    match s {
        0..60 => format!("{s}s"),
        60..3600 => format!("{}m {:02}s", s / 60, s % 60),
        _ => format!("{}h {:02}m {:02}s", s / 3600, (s / 60) % 60, s % 60),
    }
}

pub fn init() {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::System::Console::{
            ENABLE_VIRTUAL_TERMINAL_PROCESSING, GetConsoleMode, GetStdHandle, STD_OUTPUT_HANDLE,
            SetConsoleMode,
        };

        let handle = GetStdHandle(STD_OUTPUT_HANDLE);
        let mut mode = 0;

        if GetConsoleMode(handle, &mut mode) != 0 {
            SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
        }
    }
}
