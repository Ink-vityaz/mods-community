//! Packs every mod for players: `<id>-<version>.zip` each and `mods.json`, their list. The same files give the same
//! bytes, so a mod that did not change keeps its SHA-256.

use std::fs;
use std::path::Path;

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::{check, files_of};

/// Packs the mods of `root` into `out`, an empty or new folder; the list's text. Nothing is packed when one mod is not
/// all right.
pub fn pack(root: &Path, out: &Path) -> Result<String, Vec<String>> {
    let problems = check(root, None);
    if !problems.is_empty() {
        return Err(problems);
    }
    let one = |error: String| vec![error];
    if fs::read_dir(out).is_ok_and(|mut entries| entries.next().is_some()) {
        return Err(one(format!("{} is not empty", out.display())));
    }
    fs::create_dir_all(out).map_err(|error| one(error.to_string()))?;
    let withdrawn = withdrawn(root).map_err(one)?;
    let mut listed = Vec::new();
    let mut folders: Vec<_> = fs::read_dir(root.join("mods")).map_err(|error| one(error.to_string()))?.flatten().map(|entry| entry.path()).collect();
    folders.sort();
    for folder in folders {
        let id = folder.file_name().unwrap_or_default().to_string_lossy().to_string();
        if withdrawn.iter().any(|entry| entry["id"] == id.as_str()) {
            continue;
        }
        let bundle = read(&folder.join("bundle.json")).map_err(one)?;
        let listing = read(&folder.join("listing.json")).map_err(one)?;
        let mut entries = Vec::new();
        for name in files_of(&folder) {
            let data = fs::read(folder.join(&name)).map_err(|error| one(format!("{id}/{name}: {error}")))?;
            entries.push((format!("{id}/{name}"), data));
        }
        let bytes = zip(&entries);
        let file = format!("{id}-{}.zip", bundle["version"].as_str().unwrap_or_default());
        fs::write(out.join(&file), &bytes).map_err(|error| one(format!("{file}: {error}")))?;
        let has_theme = bundle["content"].get("ui/themes").is_some();
        let has_panel = bundle["modules"].as_array().is_some_and(|modules| !modules.is_empty());
        let kinds: Vec<&str> = [("panel", has_panel), ("theme", has_theme)].iter().filter(|(_, has)| *has).map(|(kind, _)| *kind).collect();
        listed.push(json!({
            "id": id, "name": bundle["name"], "version": bundle["version"], "game": bundle["game"],
            "author": listing["author"], "about": listing["about"], "kinds": kinds,
            "file": file, "size": bytes.len(), "sha256": sha256(&bytes),
        }));
    }
    let text = serde_json::to_string_pretty(&json!({"format": 1, "mods": listed, "withdrawn": withdrawn})).map_err(|error| one(error.to_string()))? + "\n";
    fs::write(out.join("mods.json"), &text).map_err(|error| one(error.to_string()))?;
    Ok(text)
}

fn withdrawn(root: &Path) -> Result<Vec<Value>, String> {
    let list = read(&root.join("withdrawn.json"))?;
    let entries = list["withdrawn"].as_array().cloned().ok_or("withdrawn.json: no list \"withdrawn\"")?;
    if entries.iter().any(|entry| !entry["id"].is_string() || !entry["why"].is_string()) {
        return Err("withdrawn.json: each entry has an \"id\" and a \"why\"".into());
    }
    Ok(entries)
}

fn read(path: &Path) -> Result<Value, String> {
    let text = fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    serde_json::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))
}

/// The SHA-256 of `bytes`, in hexadecimal.
pub fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect()
}

/// A .zip of stored entries dated 1980-01-01, their names in UTF-8.
pub fn zip(entries: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut all = Vec::new();
    let mut list = Vec::new();
    for (name, data) in entries {
        let offset = all.len() as u32;
        let mut common = Vec::new();
        for number in [20u16, 0x0800, 0, 0, 0x21] {
            common.extend(number.to_le_bytes());
        }
        for number in [crc32fast::hash(data), data.len() as u32, data.len() as u32] {
            common.extend(number.to_le_bytes());
        }
        common.extend((name.len() as u16).to_le_bytes());
        common.extend(0u16.to_le_bytes());
        all.extend(b"PK\x03\x04");
        all.extend(&common);
        all.extend(name.as_bytes());
        all.extend(data);
        list.extend(b"PK\x01\x02\x14\x00");
        list.extend(&common);
        list.extend([0u8; 10]);
        list.extend(offset.to_le_bytes());
        list.extend(name.as_bytes());
    }
    let start = all.len() as u32;
    all.extend(&list);
    all.extend(b"PK\x05\x06\x00\x00\x00\x00");
    all.extend((entries.len() as u16).to_le_bytes());
    all.extend((entries.len() as u16).to_le_bytes());
    all.extend((list.len() as u32).to_le_bytes());
    all.extend(start.to_le_bytes());
    all.extend([0u8; 2]);
    all
}
