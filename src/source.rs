use crate::logic::color::{Color, Palette};
use crate::logic::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

pub fn detect() -> Result<Palette, Error> {
    let term = std::env::var("TERM").unwrap_or_default();
    let in_kitty = env_path("KITTY_WINDOW_ID").is_some() || term == "xterm-kitty";

    let result = if term == "linux" {
        linux_console()
    } else if in_kitty {
        kitty()
    } else if cfg!(unix) && (term.is_empty() || term == "dumb") {
        Err("TERM is unset or \"dumb\", so the terminal cannot report a palette".to_string())
    } else {
        osc4_query()
    };
    result.map_err(|reason| {
        Error::runtime(format!("current terminal palette is unavailable: {reason}"))
    })
}

fn env_path(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

const VT_DIR: &str = "/sys/module/vt/parameters";

fn linux_console() -> Result<Palette, String> {
    let read = |name: &str| -> Result<[u8; 16], String> {
        let path = format!("{VT_DIR}/{name}");
        let text = fs::read_to_string(&path).map_err(|e| format!("cannot read {path}: {e}"))?;
        parse_vt_channels(&text).map_err(|e| format!("{path}: {e}"))
    };
    let (r, g, b) = (
        read("default_red")?,
        read("default_grn")?,
        read("default_blu")?,
    );
    Ok(Palette {
        colors: std::array::from_fn(|i| Color::from_srgb8(r[i], g[i], b[i])),
    })
}

fn parse_vt_channels(text: &str) -> Result<[u8; 16], String> {
    let values: Vec<u8> = text
        .trim()
        .split(',')
        .map(|t| {
            t.trim()
                .parse::<u8>()
                .map_err(|_| format!("invalid channel value '{}'", t.trim()))
        })
        .collect::<Result<_, _>>()?;
    values
        .try_into()
        .map_err(|v: Vec<u8>| format!("expected 16 values, found {}", v.len()))
}

const MAX_INCLUDE_DEPTH: u32 = 8;

#[derive(Default)]
struct KittyScan {
    colors: [Option<Color>; 16],
    ignored: Vec<String>,
}

fn kitty() -> Result<Palette, String> {
    let dir = kitty_config_dir()?;
    let main = dir.join("kitty.conf");
    if !main.is_file() {
        return Err(format!("kitty config not found at {}", main.display()));
    }
    let mut scan = KittyScan::default();
    scan_kitty_file(&main, &dir, 0, &mut scan)?;

    let missing: Vec<String> = (0..16)
        .filter(|&i| scan.colors[i].is_none())
        .map(|i| format!("color{i}"))
        .collect();
    if !missing.is_empty() {
        let mut msg = format!("{} does not set {}", main.display(), missing.join(", "));
        if !scan.ignored.is_empty() {
            msg.push_str(&format!(
                " (unsupported directives were ignored: {})",
                scan.ignored.join(", ")
            ));
        }
        return Err(msg);
    }
    Ok(Palette {
        colors: scan.colors.map(|c| c.unwrap()),
    })
}

fn kitty_config_dir() -> Result<PathBuf, String> {
    if let Some(d) = env_path("KITTY_CONFIG_DIRECTORY") {
        return Ok(d);
    }
    if let Some(d) = env_path("XDG_CONFIG_HOME") {
        return Ok(d.join("kitty"));
    }
    let home = env_path("HOME").ok_or("HOME is not set")?;
    Ok(home.join(".config/kitty"))
}

fn scan_kitty_file(
    path: &Path,
    dir: &Path,
    depth: u32,
    scan: &mut KittyScan,
) -> Result<(), String> {
    if depth > MAX_INCLUDE_DEPTH {
        return Err(format!("includes nested too deeply at {}", path.display()));
    }
    let text =
        fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    scan_kitty_text(&text, path, dir, depth, scan)
}

fn scan_kitty_text(
    text: &str,
    path: &Path,
    dir: &Path,
    depth: u32,
    scan: &mut KittyScan,
) -> Result<(), String> {
    for (n, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, value) = match line.split_once(char::is_whitespace) {
            Some((k, v)) => (k, v.trim()),
            None => (line, ""),
        };
        match key {
            "include" => {
                let target = resolve_include(value, dir);
                if target.is_file() {
                    scan_kitty_file(&target, dir, depth + 1, scan)?;
                }
            }
            "globinclude" | "envinclude" | "geninclude" => {
                if !scan.ignored.iter().any(|k| k == key) {
                    scan.ignored.push(key.to_string());
                }
            }
            _ => {
                let index = key
                    .strip_prefix("color")
                    .filter(|n| !n.starts_with('0') || *n == "0")
                    .and_then(|n| n.parse::<usize>().ok())
                    .filter(|&i| i < 16);
                if let Some(i) = index {
                    let here = |e: String| format!("{}:{}: {e}", path.display(), n + 1);
                    if !value.starts_with('#') {
                        return Err(here(format!(
                            "unsupported color value '{value}' (expected #rrggbb)"
                        )));
                    }
                    scan.colors[i] = Some(
                        crate::logic::parse::parse_literal(value)
                            .map_err(|e| here(e.to_string()))?,
                    );
                }
            }
        }
    }
    Ok(())
}

fn resolve_include(value: &str, dir: &Path) -> PathBuf {
    if let Some(rest) = value.strip_prefix("~/") {
        if let Some(home) = env_path("HOME") {
            return home.join(rest);
        }
    }
    let p = Path::new(value);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        dir.join(p)
    }
}

#[cfg(not(unix))]
fn osc4_query() -> Result<Palette, String> {
    Err("querying the palette is only supported on Unix terminals".to_string())
}

#[cfg(unix)]
const OSC4_QUERY: &str = concat!(
    "\x1b]4;0;?\x07",
    "\x1b]4;1;?\x07",
    "\x1b]4;2;?\x07",
    "\x1b]4;3;?\x07",
    "\x1b]4;4;?\x07",
    "\x1b]4;5;?\x07",
    "\x1b]4;6;?\x07",
    "\x1b]4;7;?\x07",
    "\x1b]4;8;?\x07",
    "\x1b]4;9;?\x07",
    "\x1b]4;10;?\x07",
    "\x1b]4;11;?\x07",
    "\x1b]4;12;?\x07",
    "\x1b]4;13;?\x07",
    "\x1b]4;14;?\x07",
    "\x1b]4;15;?\x07",
);

#[cfg(unix)]
fn osc4_query() -> Result<Palette, String> {
    use std::fs::OpenOptions;
    use std::io::{Read, Write};
    use std::time::{Duration, Instant};

    let mut tty = OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")
        .map_err(|e| format!("cannot open /dev/tty: {e}"))?;
    let _restore = raw::enter(&tty)?;

    tty.write_all(OSC4_QUERY.as_bytes())
        .and_then(|()| tty.flush())
        .map_err(|e| format!("cannot write to the terminal: {e}"))?;

    let deadline = Instant::now() + Duration::from_secs(2);
    let (mut buf, mut chunk) = (Vec::with_capacity(1024), [0u8; 1024]);
    loop {
        let n = tty
            .read(&mut chunk)
            .map_err(|e| format!("cannot read from the terminal: {e}"))?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
        if parse_osc4_replies(&buf).iter().all(Option::is_some) || Instant::now() > deadline {
            break;
        }
    }

    let colors = parse_osc4_replies(&buf);
    match colors.iter().flatten().count() {
        16 => Ok(Palette {
            colors: colors.map(|c| c.unwrap()),
        }),
        0 => Err("the terminal did not answer OSC 4 color queries".to_string()),
        n => Err(format!(
            "the terminal answered only {n} of 16 OSC 4 color queries"
        )),
    }
}

#[cfg(all(
    unix,
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
mod raw {
    use std::fs::File;
    use std::io;
    use std::os::fd::AsRawFd;

    const ICANON: u32 = 0o2;
    const ECHO: u32 = 0o10;
    const VTIME: usize = 5;
    const VMIN: usize = 6;
    const TCSANOW: i32 = 0;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Termios {
        c_iflag: u32,
        c_oflag: u32,
        c_cflag: u32,
        c_lflag: u32,
        c_line: u8,
        c_cc: [u8; 32],
        c_ispeed: u32,
        c_ospeed: u32,
    }

    const _: () = assert!(std::mem::size_of::<Termios>() == 60);

    unsafe extern "C" {
        fn tcgetattr(fd: i32, termios: *mut Termios) -> i32;
        fn tcsetattr(fd: i32, optional_actions: i32, termios: *const Termios) -> i32;
        fn tcgetpgrp(fd: i32) -> i32;
        fn getpgrp() -> i32;
    }

    pub struct Guard {
        fd: i32,
        saved: Termios,
    }

    pub fn enter(tty: &File) -> Result<Guard, String> {
        let fd = tty.as_raw_fd();
        if unsafe { tcgetpgrp(fd) != getpgrp() } {
            return Err("not running in the terminal's foreground process group".to_string());
        }
        let mut saved: Termios = unsafe { std::mem::zeroed() };
        if unsafe { tcgetattr(fd, &mut saved) } != 0 {
            return Err(format!(
                "cannot read terminal settings: {}",
                io::Error::last_os_error()
            ));
        }
        let mut changed = saved;
        changed.c_lflag &= !(ICANON | ECHO);
        changed.c_cc[VMIN] = 0;
        changed.c_cc[VTIME] = 5;
        if unsafe { tcsetattr(fd, TCSANOW, &changed) } != 0 {
            return Err(format!(
                "cannot change terminal settings: {}",
                io::Error::last_os_error()
            ));
        }
        Ok(Guard { fd, saved })
    }

    impl Drop for Guard {
        fn drop(&mut self) {
            unsafe {
                tcsetattr(self.fd, TCSANOW, &self.saved);
            }
        }
    }
}

#[cfg(all(
    unix,
    not(all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    ))
))]
mod raw {
    use std::fs::File;
    use std::process::{Command, Stdio};

    pub struct Guard {
        tty: File,
        saved: String,
    }

    fn stty(tty: &File, args: &[&str]) -> Result<String, String> {
        let stdin = tty.try_clone().map_err(|e| e.to_string())?;
        let out = Command::new("stty")
            .args(args)
            .stdin(Stdio::from(stdin))
            .stderr(Stdio::null())
            .output()
            .map_err(|e| format!("cannot run stty: {e}"))?;
        if !out.status.success() {
            return Err(format!("`stty {}` failed", args.join(" ")));
        }
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }

    pub fn enter(tty: &File) -> Result<Guard, String> {
        let tty = tty
            .try_clone()
            .map_err(|e| format!("cannot open /dev/tty: {e}"))?;
        let saved = stty(&tty, &["-g"])?;
        let guard = Guard { tty, saved };
        stty(&guard.tty, &["-icanon", "-echo", "min", "0", "time", "5"])?;
        Ok(guard)
    }

    impl Drop for Guard {
        fn drop(&mut self) {
            let _ = stty(&self.tty, &[self.saved.as_str()]);
        }
    }
}

#[cfg(unix)]
fn parse_osc4_replies(buf: &[u8]) -> [Option<Color>; 16] {
    let text = String::from_utf8_lossy(buf);
    let mut out = [None; 16];
    for part in text.split("\x1b]4;").skip(1) {
        let Some((index, rest)) = part.split_once(';') else {
            continue;
        };
        let Ok(i) = index.parse::<usize>() else {
            continue;
        };
        if i >= 16 {
            continue;
        }
        let spec = rest.split(['\x07', '\x1b']).next().unwrap_or("");
        if let Some(color) = parse_x11_rgb(spec) {
            out[i] = Some(color);
        }
    }
    out
}

#[cfg(unix)]
fn parse_x11_rgb(spec: &str) -> Option<Color> {
    let mut parts = spec.strip_prefix("rgb:")?.split('/');
    let mut v = [0.0f64; 3];
    for slot in &mut v {
        let digits = parts.next()?;
        if digits.is_empty() || digits.len() > 4 {
            return None;
        }
        let n = u32::from_str_radix(digits, 16).ok()?;
        *slot = n as f64 / ((1u32 << (4 * digits.len())) - 1) as f64;
    }
    if parts.next().is_some() {
        return None;
    }
    Some(Color::from_encoded(v[0], v[1], v[2]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_vt_channel_lists() {
        let ok = "0,170,0,170,0,170,0,170,85,255,85,255,85,255,85,255\n";
        let v = parse_vt_channels(ok).unwrap();
        assert_eq!((v[0], v[1], v[9], v[15]), (0, 170, 255, 255));
        assert!(
            parse_vt_channels("0,1,2")
                .unwrap_err()
                .contains("expected 16 values, found 3")
        );
        assert!(parse_vt_channels("0,1,300,3,4,5,6,7,8,9,10,11,12,13,14,15").is_err());
        assert!(parse_vt_channels("").is_err());
    }

    fn scan(text: &str, dir: &Path) -> Result<KittyScan, String> {
        let mut s = KittyScan::default();
        scan_kitty_text(text, &dir.join("kitty.conf"), dir, 0, &mut s)?;
        Ok(s)
    }

    fn hex(c: Option<Color>) -> String {
        let (r, g, b) = c.unwrap().srgb8().unwrap();
        format!("#{r:02x}{g:02x}{b:02x}")
    }

    #[test]
    fn kitty_colors_comments_and_overrides() {
        let s = scan(
            "# color1 #111111\ncolor1 #aabbcc\ncolor9\t#112233\nfont_size 12\ncolor1 #ddeeff\ncolor05 #000000\n",
            Path::new("/nonexistent"),
        )
        .unwrap();
        assert_eq!(hex(s.colors[1]), "#ddeeff");
        assert_eq!(hex(s.colors[9]), "#112233");
        assert!(s.colors[5].is_none());
    }

    #[test]
    fn kitty_rejects_unsupported_color_syntax() {
        let e = scan("color1 red\n", Path::new("/nonexistent"))
            .err()
            .unwrap();
        assert!(e.contains("kitty.conf:1") && e.contains("unsupported color value 'red'"));
    }

    #[test]
    fn kitty_includes_are_spliced_in_order() {
        let dir = std::env::temp_dir().join(format!("palette-test-{}", std::process::id()));
        fs::create_dir_all(dir.join("themes")).unwrap();
        fs::write(
            dir.join("themes/t.conf"),
            "color0 #000001\ncolor1 #000002\n",
        )
        .unwrap();
        let s = scan("color0 #ffffff\ninclude themes/t.conf\ncolor1 #00ff00\nglobinclude x/*.conf\ninclude missing.conf\n", &dir).unwrap();
        assert_eq!(hex(s.colors[0]), "#000001");
        assert_eq!(hex(s.colors[1]), "#00ff00");
        assert_eq!(s.ignored, ["globinclude"]);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn kitty_include_cycles_terminate() {
        let dir = std::env::temp_dir().join(format!("palette-cycle-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("a.conf"), "include a.conf\n").unwrap();
        let e = scan("include a.conf\n", &dir).err().unwrap();
        assert!(e.contains("nested too deeply"));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn parses_osc4_replies() {
        let mut buf = Vec::new();
        buf.extend_from_slice(b"\x1b]4;0;rgb:2424/2121/2020\x07");
        buf.extend_from_slice(b"\x1b]4;15;rgb:f4/ee/dc\x1b\\");
        buf.extend_from_slice(b"\x1b]4;16;rgb:ffff/ffff/ffff\x07");
        buf.extend_from_slice(b"\x1b]4;3;rgb:zz/00/00\x07");
        let c = parse_osc4_replies(&buf);
        assert_eq!(c[0].unwrap().srgb8(), Some((0x24, 0x21, 0x20)));
        assert_eq!(c[15].unwrap().srgb8(), Some((0xf4, 0xee, 0xdc)));
        assert_eq!(c.iter().flatten().count(), 2);
        assert!(parse_x11_rgb("rgb:1/2").is_none());
        assert!(parse_x11_rgb("rgb:1/2/3/4").is_none());
        assert!(parse_x11_rgb("#ffffff").is_none());
    }
}
