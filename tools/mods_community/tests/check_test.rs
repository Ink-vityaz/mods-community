use std::fs;
use std::path::{Path, PathBuf};

use mods_community::pack::pack;
use mods_community::{check, check_mod, is_id, is_version, not_raised, outside, picture_size};

struct Scratch(PathBuf);

impl Scratch {
    /// A root with one good mod, `one`.
    fn new(name: &str) -> Scratch {
        let root = std::env::temp_dir().join(format!("mods_community_{name}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let scratch = Scratch(root);
        scratch.put("bundle.json", r#"{"format": 1, "id": "one", "name": "One", "version": "1.0.0", "game": "0.29.x", "depends": {}, "modules": ["one"], "content": {}}"#);
        scratch.put("README.md", "One");
        scratch.put("listing.json", r#"{"author": "Lan", "about": "One panel"}"#);
        fs::write(scratch.0.join("withdrawn.json"), r#"{"withdrawn": []}"#).unwrap();
        scratch.put("modules/one/module.json", r#"{"id": "one", "version": "1.0.0", "client": {"panels": ["panel.json"]}}"#);
        scratch.put("modules/one/client/panel.json", r#"{"cells": [{"type": "text", "text": "Hi"}]}"#);
        scratch
    }

    fn put(&self, name: &str, text: &str) {
        let path = self.0.join("mods/one").join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn problems(&self) -> Vec<String> {
        check(&self.0, None)
    }

    fn assert_refused(&self, part: &str) {
        let problems = self.problems();
        assert!(problems.iter().any(|problem| problem.contains(part)), "{part} in {problems:?}");
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn a_good_mod_is_all_right() {
    assert_eq!(Scratch::new("good").problems(), Vec::<String>::new());
}

#[test]
fn the_mods_here_are_all_right() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    assert_eq!(check(&root, None), Vec::<String>::new());
}

#[test]
fn code_is_refused_wherever_it_is() {
    let scratch = Scratch::new("code");
    scratch.put("modules/one/client/main.gd", "extends Module");
    scratch.put("modules/one/server/main.cs", "class A {}");
    scratch.put("modules/one/module.json", r#"{"version": "1.0.0", "client": {"script": "main.gd", "needs_server": true, "buttons": [{"id": "a", "op": "inventory.sort"}]}}"#);
    for part in ["main.gd: this kind", "main.cs: this kind", "server: a C# half", "client.script", "client.needs_server", "may only open a panel"] {
        scratch.assert_refused(part);
    }
}

#[test]
fn a_panel_that_sends_a_command_is_refused() {
    let scratch = Scratch::new("button");
    scratch.put("modules/one/client/panel.json", r#"{"cells": [{"type": "button", "op": "inventory.sort"}]}"#);
    scratch.assert_refused("sends a command");
}

#[test]
fn a_theme_with_a_scene_a_font_or_a_shader_is_refused() {
    let scratch = Scratch::new("theme");
    scratch.put("bundle.json", r#"{"format": 1, "id": "one", "name": "One", "version": "1.0.0", "game": "0.29.x", "content": {"ui/themes": ["red"], "unit/skins": ["x"]}}"#);
    scratch.put("data/ui/themes/red.json", r#"{"id": "red", "decor": [], "fonts": {}, "fills": {"bar": {"type": "layers", "layers": [{"type": "shader"}]}}}"#);
    for part in ["scenes are not taken", "fonts are not taken", "shaders are not taken", "unit/skins is not taken"] {
        scratch.assert_refused(part);
    }
}

#[test]
fn a_bad_bundle_says_each_mistake() {
    let scratch = Scratch::new("bundle");
    scratch.put("bundle.json", r#"{"format": 2, "id": "two", "version": "1.0", "depends": {"a": "x"}}"#);
    for part in ["\"format\"", "\"id\"", "\"version\"", "\"name\"", "\"game\"", "\"depends\""] {
        scratch.assert_refused(part);
    }
}

#[test]
fn a_hidden_character_and_a_false_picture_are_refused() {
    let scratch = Scratch::new("hidden");
    scratch.put("README.md", "a\u{202e}b");
    scratch.put("icon.png", "not a picture");
    scratch.assert_refused("U+202E");
    scratch.assert_refused("not a PNG");
}

#[test]
fn a_large_file_is_refused() {
    let scratch = Scratch::new("large");
    scratch.put("notes.txt", &"a".repeat(600 * 1024));
    scratch.assert_refused("bytes");
}

#[test]
fn a_panel_outside_its_folder_is_not_read() {
    let scratch = Scratch::new("outside");
    scratch.put("modules/one/module.json", r#"{"version": "1.0.0", "client": {"panels": ["../../bundle.json"]}}"#);
    scratch.assert_refused("not a file of client/");
}

#[test]
fn a_contribution_changes_mods_only() {
    assert_eq!(outside("mods/one/bundle.json\n"), Vec::<String>::new());
    assert_eq!(outside(".github/workflows/check.yml\nmods/one\nREADME.md\nmods/../x/y\n").len(), 4);
}

#[test]
fn ids_versions_and_pictures() {
    assert!(is_id("my_panel2") && !is_id("_a") && !is_id("My") && !is_id("a-b") && !is_id(""));
    assert!(is_version("1.0.0") && !is_version("1.0") && !is_version("01.0.0") && !is_version("1.0.0-x"));
    assert_eq!(picture_size(b"\x89PNG"), None);
}

#[test]
fn a_folder_that_is_no_id_is_refused() {
    let scratch = Scratch::new("folder");
    assert!(check_mod(&scratch.0.join("mods/one"), "Bad Name").iter().any(|problem| problem.contains("no id")));
}

#[test]
fn a_changed_mod_needs_a_higher_version() {
    let (old, new) = (Scratch::new("base_old"), Scratch::new("base_new"));
    assert_eq!(not_raised(&new.0, &old.0), Vec::<String>::new());
    new.put("README.md", "Two");
    assert_eq!(not_raised(&new.0, &old.0).len(), 1);
    new.put("bundle.json", r#"{"format": 1, "id": "one", "name": "One", "version": "1.0.10", "game": "0.29.x", "modules": ["one"]}"#);
    assert_eq!(not_raised(&new.0, &old.0), Vec::<String>::new());
    old.put("bundle.json", r#"{"format": 1, "id": "one", "name": "One", "version": "1.0.9", "game": "0.29.x", "modules": ["one"]}"#);
    assert_eq!(not_raised(&new.0, &old.0), Vec::<String>::new());
}

#[test]
fn a_listing_says_who_and_what() {
    let scratch = Scratch::new("listing");
    scratch.put("listing.json", r#"{"author": "", "about": "a\nb"}"#);
    scratch.assert_refused("\"author\"");
    scratch.assert_refused("\"about\"");
}

#[test]
fn the_same_mods_pack_to_the_same_bytes() {
    let scratch = Scratch::new("pack");
    let (first, second) = (scratch.0.join("a"), scratch.0.join("b"));
    let list = pack(&scratch.0, &first).unwrap();
    assert_eq!(list, pack(&scratch.0, &second).unwrap());
    assert_eq!(fs::read(first.join("one-1.0.0.zip")).unwrap(), fs::read(second.join("one-1.0.0.zip")).unwrap());
    assert!(list.contains("\"author\": \"Lan\"") && list.contains("\"panel\""), "{list}");
    assert!(pack(&scratch.0, &first).is_err(), "a folder already filled is not packed into");
}

#[test]
fn a_withdrawn_mod_is_not_packed() {
    let scratch = Scratch::new("withdrawn");
    fs::write(scratch.0.join("withdrawn.json"), r#"{"withdrawn": [{"id": "one", "why": "broken"}]}"#).unwrap();
    let out = scratch.0.join("out");
    let list = pack(&scratch.0, &out).unwrap();
    assert!(!out.join("one-1.0.0.zip").exists() && list.contains("broken"));
}

#[test]
fn a_mod_that_is_not_all_right_packs_nothing() {
    let scratch = Scratch::new("pack_bad");
    scratch.put("main.gd", "x");
    let out = scratch.0.join("out");
    assert!(pack(&scratch.0, &out).is_err() && !out.exists());
}
