//! What a mod handed in must be: files that hold no code, as the README says. It only reads; nothing of a mod is run.
//! Whether the game takes the mod is the game's to say: the maintainer asks it before a release.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

pub mod pack;

/// The kinds of file a mod may hold.
pub const KINDS: [&str; 5] = ["json", "png", "csv", "md", "txt"];
/// The largest file, the largest mod (bytes) and the most files.
pub const FILE_MOST: u64 = 512 * 1024;
pub const MOD_MOST: u64 = 2 * 1024 * 1024;
pub const FILES_MOST: usize = 100;
/// The widest and tallest picture.
pub const PICTURE_MOST: u32 = 2048;
/// The kinds of content a mod here may list.
const CONTENT: [&str; 1] = ["ui/themes"];
const PNG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];

/// Every problem of every mod in `root`/mods, or of `only` one; empty when they are all right.
pub fn check(root: &Path, only: Option<&str>) -> Vec<String> {
    let mods = root.join("mods");
    let mut problems = Vec::new();
    let Ok(entries) = fs::read_dir(&mods) else { return vec!["there is no mods folder".into()] };
    let mut folders: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
    folders.sort();
    let mut seen = BTreeSet::new();
    for folder in folders {
        let id = folder.file_name().unwrap_or_default().to_string_lossy().to_string();
        if only.is_some_and(|wanted| wanted != id) {
            continue;
        }
        if !folder.is_dir() || is_link(&folder) {
            problems.push(format!("mods/{id}: only folders belong in mods/"));
            continue;
        }
        for problem in check_mod(&folder, &id) {
            problems.push(format!("mods/{id}: {problem}"));
        }
        for module in modules_of(&folder) {
            if !seen.insert(module.clone()) {
                problems.push(format!("mods/{id}: another mod has the module {module}"));
            }
        }
    }
    if let Some(wanted) = only {
        if !mods.join(wanted).is_dir() {
            problems.push(format!("there is no mods/{wanted}"));
        }
    }
    problems
}

/// The paths of `changed` a contribution may not touch: everything but files in mods/<id>/.
pub fn outside(changed: &str) -> Vec<String> {
    let mut found = Vec::new();
    for line in changed.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let mut parts = line.split('/');
        let inside = parts.next() == Some("mods") && parts.next().is_some_and(is_id) && parts.next().is_some();
        if !inside || line.contains("..") {
            found.push(line.to_string());
        }
    }
    found
}

/// The mods of `root` that differ from the same mod of `base` without a higher version.
pub fn not_raised(root: &Path, base: &Path) -> Vec<String> {
    let mut found = Vec::new();
    for entry in fs::read_dir(root.join("mods")).into_iter().flatten().flatten() {
        let id = entry.file_name().to_string_lossy().to_string();
        let (new, old) = (entry.path(), base.join("mods").join(&id));
        if !old.is_dir() || contents(&new) == contents(&old) {
            continue;
        }
        let (now, before) = (version_of(&new), version_of(&old));
        if now <= before {
            found.push(format!("mods/{id}: it changed, so \"version\" in bundle.json must be higher than it was"));
        }
    }
    found.sort();
    found
}

fn version_of(folder: &Path) -> Vec<u32> {
    let bundle = json(folder, "bundle.json", &mut Vec::new()).unwrap_or_default();
    bundle["version"].as_str().unwrap_or_default().split('.').map(|part| part.parse().unwrap_or(0)).collect()
}

fn contents(folder: &Path) -> Vec<(String, Vec<u8>)> {
    files_of(folder).into_iter().map(|name| (name.clone(), fs::read(folder.join(&name)).unwrap_or_default())).collect()
}

/// The files of the mod in `folder`, as paths from it, in order.
pub fn files_of(folder: &Path) -> Vec<String> {
    let mut files = Vec::new();
    walk(folder, "", &mut files, &mut Vec::new());
    files.sort();
    files
}

/// Whether `text` can be an id: lower case letters, digits and _, not starting with _.
pub fn is_id(text: &str) -> bool {
    !text.is_empty() && !text.starts_with('_') && text.bytes().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

/// Whether `text` is a version, three numbers.
pub fn is_version(text: &str) -> bool {
    let parts: Vec<&str> = text.split('.').collect();
    parts.len() == 3 && parts.iter().all(|part| !part.is_empty() && part.len() < 6 && part.bytes().all(|byte| byte.is_ascii_digit()) && (*part == "0" || !part.starts_with('0')))
}

/// What is wrong with the mod in `folder`.
pub fn check_mod(folder: &Path, id: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if !is_id(id) {
        problems.push("the folder's name is no id (lower case letters, digits and _)".into());
    }
    let mut files = Vec::new();
    walk(folder, "", &mut files, &mut problems);
    if files.len() > FILES_MOST {
        problems.push(format!("{} files, {FILES_MOST} at most", files.len()));
    }
    let mut all = 0;
    for name in &files {
        all += check_file(folder, name, &mut problems);
    }
    if all > MOD_MOST {
        problems.push(format!("{all} bytes in all, {MOD_MOST} at most"));
    }
    if !files.iter().any(|name| name == "README.md") {
        problems.push("there is no README.md".into());
    }
    if let Some(listing) = json(folder, "listing.json", &mut problems) {
        for (key, most) in [("author", 40), ("about", 160)] {
            if !listing.get(key).and_then(Value::as_str).is_some_and(|text| !text.trim().is_empty() && text.chars().count() <= most && !text.contains('\n')) {
                problems.push(format!("listing.json: \"{key}\" is missing, longer than {most} letters or of two lines"));
            }
        }
    }
    if let Some(bundle) = json(folder, "bundle.json", &mut problems) {
        check_bundle(folder, id, &bundle, &mut problems);
    }
    problems
}

fn check_bundle(folder: &Path, id: &str, bundle: &Value, problems: &mut Vec<String>) {
    if bundle.get("format") != Some(&Value::from(1)) {
        problems.push("bundle.json: \"format\" is not 1".into());
    }
    if bundle.get("id").and_then(Value::as_str) != Some(id) {
        problems.push("bundle.json: \"id\" is not the folder's name".into());
    }
    if !bundle.get("version").and_then(Value::as_str).is_some_and(is_version) {
        problems.push("bundle.json: \"version\" is not three numbers, 1.0.0".into());
    }
    if !bundle.get("name").and_then(Value::as_str).is_some_and(|name| !name.trim().is_empty() && name.chars().count() <= 40) {
        problems.push("bundle.json: \"name\" is missing or longer than 40 letters".into());
    }
    if !bundle.get("game").and_then(Value::as_str).is_some_and(|game| !game.is_empty() && game != "x") {
        problems.push("bundle.json: \"game\" does not say which game versions, 0.29.x".into());
    }
    if bundle.get("depends").and_then(Value::as_object).is_some_and(|depends| !depends.is_empty()) {
        problems.push("bundle.json: \"depends\" must be empty here".into());
    }
    for module in modules_of(folder) {
        check_module(folder, &module, problems);
    }
    let Some(content) = bundle.get("content").and_then(Value::as_object) else { return };
    for (kind, ids) in content {
        if !CONTENT.contains(&kind.as_str()) {
            problems.push(format!("bundle.json: content {kind} is not taken here"));
            continue;
        }
        for theme in ids.as_array().into_iter().flatten().filter_map(Value::as_str) {
            if !is_id(theme) {
                problems.push(format!("bundle.json: content {kind} holds \"{theme}\", no id"));
                continue;
            }
            check_theme(folder, theme, problems);
        }
    }
}

// A module here is panels: no script, no server half, no command sent.
fn check_module(folder: &Path, module: &str, problems: &mut Vec<String>) {
    let place = format!("modules/{module}");
    if folder.join(&place).join("server").exists() {
        problems.push(format!("{place}/server: a C# half is not taken here"));
    }
    let Some(manifest) = json(folder, &format!("{place}/module.json"), problems) else { return };
    let client = manifest.get("client").cloned().unwrap_or_default();
    for (key, why) in [("script", "a script is code"), ("needs_server", "players' games load no C# half")] {
        if client.get(key).is_some_and(|value| value != &Value::Bool(false) && value != &Value::from("")) {
            problems.push(format!("{place}/module.json: client.{key} is not taken here ({why})"));
        }
    }
    for button in client.get("buttons").and_then(Value::as_array).into_iter().flatten() {
        if button.get("op").is_some() || button.get("call").is_some() {
            problems.push(format!("{place}/module.json: a button may only open a panel here"));
        }
    }
    for panel in client.get("panels").and_then(Value::as_array).into_iter().flatten() {
        let name = panel.as_str().unwrap_or_default();
        if name.is_empty() || name.contains(['/', '\\', ':']) {
            problems.push(format!("{place}/module.json: the panel \"{name}\" is not a file of client/"));
            continue;
        }
        let Some(spec) = json(folder, &format!("{place}/client/{name}"), problems) else { continue };
        for cell in spec.get("cells").and_then(Value::as_array).into_iter().flatten() {
            if cell.get("type").and_then(Value::as_str) == Some("button") {
                problems.push(format!("{place}/client/{name}: a cell of type button sends a command, not taken here"));
            }
        }
    }
}

// A theme here is colours, sizes, pictures and plain fills: no scene, no shader, no font.
fn check_theme(folder: &Path, theme: &str, problems: &mut Vec<String>) {
    let name = format!("data/ui/themes/{theme}.json");
    let Some(data) = json(folder, &name, problems) else { return };
    for (key, why) in [("decor", "scenes"), ("fonts", "fonts")] {
        if data.get(key).is_some() {
            problems.push(format!("{name}: {why} are not taken here yet (\"{key}\")"));
        }
    }
    if data.get("fills").is_some_and(has_shader) {
        problems.push(format!("{name}: shaders are not taken here yet"));
    }
}

fn has_shader(value: &Value) -> bool {
    match value {
        Value::Object(object) => object.get("type").and_then(Value::as_str) == Some("shader") || object.values().any(has_shader),
        Value::Array(list) => list.iter().any(has_shader),
        _ => false,
    }
}

fn modules_of(folder: &Path) -> Vec<String> {
    let mut found: Vec<String> = fs::read_dir(folder.join("modules"))
        .map(|entries| entries.flatten().filter(|entry| entry.path().is_dir()).map(|entry| entry.file_name().to_string_lossy().to_string()).collect())
        .unwrap_or_default();
    found.sort();
    found
}

// The file's size; what is wrong with it is added.
fn check_file(folder: &Path, name: &str, problems: &mut Vec<String>) -> u64 {
    let kind = name.rsplit_once('.').map(|(_, kind)| kind.to_string()).unwrap_or_default();
    if !KINDS.contains(&kind.as_str()) {
        problems.push(format!("{name}: this kind of file is not taken here ({})", KINDS.join(", ")));
        return 0;
    }
    let size = fs::metadata(folder.join(name)).map(|data| data.len()).unwrap_or(0);
    if size > FILE_MOST {
        problems.push(format!("{name}: {size} bytes, {FILE_MOST} at most"));
        return size;
    }
    let Ok(bytes) = fs::read(folder.join(name)) else {
        problems.push(format!("{name}: does not read"));
        return size;
    };
    if kind == "png" {
        match picture_size(&bytes) {
            None => problems.push(format!("{name}: not a PNG picture")),
            Some((width, height)) if width > PICTURE_MOST || height > PICTURE_MOST => {
                problems.push(format!("{name}: {width} x {height}, {PICTURE_MOST} at most"));
            }
            Some(_) => {}
        }
        return size;
    }
    match String::from_utf8(bytes) {
        Err(_) => problems.push(format!("{name}: not UTF-8 text")),
        Ok(text) => {
            if let Some(hidden) = text.chars().find(|letter| is_hidden(*letter)) {
                problems.push(format!("{name}: holds the hidden character U+{:04X}", hidden as u32));
            }
        }
    }
    size
}

/// The width and height a PNG's header says; `None` when it is none.
pub fn picture_size(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 33 || bytes[..8] != PNG || &bytes[12..16] != b"IHDR" {
        return None;
    }
    let number = |at: usize| u32::from_be_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]);
    let (width, height) = (number(16), number(20));
    (width > 0 && height > 0).then_some((width, height))
}

/// Whether a reader would not see `letter`: control, zero-width, direction and tag characters.
pub fn is_hidden(letter: char) -> bool {
    matches!(letter as u32,
        0x00..=0x08 | 0x0b | 0x0c | 0x0e..=0x1f | 0x7f..=0x9f | 0xad | 0x61c | 0x180e | 0x200b..=0x200f
        | 0x2028..=0x202e | 0x2060..=0x206f | 0xfeff | 0xe0000..=0xe007f)
}

fn json(folder: &Path, name: &str, problems: &mut Vec<String>) -> Option<Value> {
    match fs::read_to_string(folder.join(name)) {
        Err(_) => {
            problems.push(format!("{name} is missing"));
            None
        }
        Ok(text) => match serde_json::from_str::<Value>(&text) {
            Ok(value) if value.is_object() => Some(value),
            Ok(_) => {
                problems.push(format!("{name}: not an object"));
                None
            }
            Err(error) => {
                problems.push(format!("{name}: {error}"));
                None
            }
        },
    }
}

fn walk(folder: &Path, prefix: &str, files: &mut Vec<String>, problems: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(folder.join(prefix)) else { return };
    let mut entries: Vec<_> = entries.flatten().collect();
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let name = format!("{prefix}{}", entry.file_name().to_string_lossy());
        if is_link(&entry.path()) {
            problems.push(format!("{name}: a link is not taken here"));
        } else if entry.path().is_dir() {
            if name.matches('/').count() < 8 {
                walk(folder, &format!("{name}/"), files, problems);
            } else {
                problems.push(format!("{name}: too deep"));
            }
        } else {
            files.push(name);
        }
    }
}

fn is_link(path: &Path) -> bool {
    fs::symlink_metadata(path).map(|data| data.file_type().is_symlink()).unwrap_or(false)
}
