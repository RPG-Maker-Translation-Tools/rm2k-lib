use rm2k::{Ctx, Engine, NullSink, event::CommandLists, file};
use std::path::Path;

/// `ShowMessage` and its continuation lines - the two codes that carry displayed text.
mod code {
    /// Begins a message box.
    pub const SHOW_MESSAGE: i32 = 10110;
    /// Continues the message box begun by the preceding `SHOW_MESSAGE`.
    pub const SHOW_MESSAGE_NEXT: i32 = 20110;
    /// One selectable choice.
    pub const SHOW_CHOICE_OPTION: i32 = 20140;
}

/// Render bytes readably without pretending to know their encoding.
fn escape(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    for &byte in bytes {
        match byte {
            b'\\' => out.push_str("\\\\"),
            0x20..=0x7E => out.push(byte as char),
            _ => out.push_str(&format!("\\x{byte:02x}")),
        }
    }
    out
}

fn dump_map(path: &Path, totals: &mut (usize, usize)) {
    let Ok(bytes) = std::fs::read(path) else { return };
    let Ok(loaded) = file::load_map(&bytes) else {
        eprintln!("could not parse {}", path.display());
        return;
    };

    // Re-derive the body span: everything after the length-prefixed magic.
    let body = &bytes[1 + loaded.header.len()..];

    let mut sink = NullSink;
    let mut ctx = Ctx::new(Engine::R2K, &mut sink);
    let mut lists = CommandLists::in_map(body, 0, &mut ctx);

    let name = path.file_name().unwrap_or_default().to_string_lossy();
    while let Some(list) = lists.next(&mut ctx) {
        let mut commands = list.commands();
        while let Some(command) = commands.next(&mut ctx) {
            let kind = match command.code {
                code::SHOW_MESSAGE => "msg",
                code::SHOW_MESSAGE_NEXT => "  +",
                code::SHOW_CHOICE_OPTION => "opt",
                _ => continue,
            };
            totals.0 += 1;
            totals.1 += command.string.len();
            println!(
                "{name}:{ev}:{page} {kind} {text}",
                ev = list.owner_id,
                page = list.page_id,
                text = escape(command.string)
            );
        }
    }
}

fn main() {
    let Some(root) = std::env::args().nth(1) else {
        eprintln!("usage: dump_text <game directory>");
        std::process::exit(2);
    };

    let Ok(entries) = std::fs::read_dir(&root) else {
        eprintln!("cannot read {root}");
        std::process::exit(1);
    };

    let mut maps: Vec<_> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .map(str::to_ascii_lowercase)
                .as_deref()
                == Some("lmu")
        })
        .collect();
    maps.sort();

    let mut totals = (0usize, 0usize);
    for map in &maps {
        dump_map(map, &mut totals);
    }

    eprintln!(
        "\n{} strings, {} bytes of text, from {} maps",
        totals.0,
        totals.1,
        maps.len()
    );
}
