//! File associations, written under HKCU so no admin rights are needed.
//! Since Windows 8 a program cannot make itself the default viewer, so after
//! registering, the Default apps page opens and the user picks Guren there.

use std::io;
use std::path::Path;

use winreg::RegKey;
use winreg::enums::HKEY_CURRENT_USER;

use crate::folder::EXTENSIONS;

const PROG_ID: &str = "Guren.Image";
const APP_NAME: &str = "Guren";
const CAPABILITIES: &str = r"Software\Guren\Capabilities";

struct Entry {
    key: String,
    name: String,
    value: String,
}

fn entry(key: impl Into<String>, name: impl Into<String>, value: impl Into<String>) -> Entry {
    Entry {
        key: key.into(),
        name: name.into(),
        value: value.into(),
    }
}

fn entries(exe: &Path) -> Vec<Entry> {
    let exe = exe.display();
    let class = format!(r"Software\Classes\{PROG_ID}");
    let mut v = vec![
        entry(&class, "", "Image"),
        entry(format!(r"{class}\DefaultIcon"), "", format!("\"{exe}\",0")),
        entry(
            format!(r"{class}\shell\open\command"),
            "",
            format!("\"{exe}\" \"%1\""),
        ),
        entry(CAPABILITIES, "ApplicationName", APP_NAME),
        entry(CAPABILITIES, "ApplicationDescription", "Photo viewer"),
        entry(r"Software\RegisteredApplications", APP_NAME, CAPABILITIES),
    ];
    for ext in EXTENSIONS {
        v.push(entry(
            format!(r"Software\Classes\.{ext}\OpenWithProgids"),
            PROG_ID,
            "",
        ));
        v.push(entry(
            format!(r"{CAPABILITIES}\FileAssociations"),
            format!(".{ext}"),
            PROG_ID,
        ));
    }
    v
}

fn command_key() -> String {
    format!(r"Software\Classes\{PROG_ID}\shell\open\command")
}

pub fn register(exe: &Path) -> io::Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    for e in entries(exe) {
        let (key, _) = hkcu.create_subkey(&e.key)?;
        key.set_value(&e.name, &e.value)?;
    }
    notify_shell();
    Ok(())
}

pub fn unregister() -> io::Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    ignore_missing(hkcu.delete_subkey_all(format!(r"Software\Classes\{PROG_ID}")))?;
    ignore_missing(hkcu.delete_subkey_all(r"Software\Guren"))?;
    if let Ok(apps) = hkcu.open_subkey_with_flags(
        r"Software\RegisteredApplications",
        winreg::enums::KEY_SET_VALUE,
    ) {
        ignore_missing(apps.delete_value(APP_NAME))?;
    }
    for ext in EXTENSIONS {
        let path = format!(r"Software\Classes\.{ext}\OpenWithProgids");
        if let Ok(key) = hkcu.open_subkey_with_flags(&path, winreg::enums::KEY_SET_VALUE) {
            ignore_missing(key.delete_value(PROG_ID))?;
        }
    }
    notify_shell();
    Ok(())
}

pub fn is_registered() -> bool {
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(command_key())
        .is_ok()
}

/// A portable exe gets moved around. If the registered command points at an
/// old location, opening a photo from Explorer would fail.
pub fn refresh_if_moved(exe: &Path) -> io::Result<()> {
    let Ok(key) = RegKey::predef(HKEY_CURRENT_USER).open_subkey(command_key()) else {
        return Ok(());
    };
    let current: String = key.get_value("")?;
    if current != format!("\"{}\" \"%1\"", exe.display()) {
        register(exe)?;
    }
    Ok(())
}

pub fn open_default_apps() {
    let url = format!("ms-settings:defaultapps?registeredAppUser={APP_NAME}");
    let wide = |s: &str| s.encode_utf16().chain(Some(0)).collect::<Vec<u16>>();
    let (verb, url) = (wide("open"), wide(&url));
    unsafe {
        windows_sys::Win32::UI::Shell::ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            url.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL,
        );
    }
}

fn notify_shell() {
    use windows_sys::Win32::UI::Shell::{SHCNE_ASSOCCHANGED, SHCNF_IDLIST, SHChangeNotify};
    unsafe {
        SHChangeNotify(
            SHCNE_ASSOCCHANGED as i32,
            SHCNF_IDLIST,
            std::ptr::null(),
            std::ptr::null(),
        );
    }
}

fn ignore_missing(r: io::Result<()>) -> io::Result<()> {
    match r {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_quotes_paths_with_spaces() {
        let exe = Path::new(r"C:\Program Files\Guren\guren.exe");
        let all = entries(exe);
        let cmd = all.iter().find(|e| e.key == command_key()).unwrap();
        assert_eq!(cmd.value, r#""C:\Program Files\Guren\guren.exe" "%1""#);
    }

    #[test]
    fn every_extension_is_offered() {
        let all = entries(Path::new(r"C:\guren.exe"));
        for ext in EXTENSIONS {
            let dotted = format!(".{ext}");
            assert!(
                all.iter()
                    .any(|e| e.key.ends_with("FileAssociations") && e.name == dotted),
                "{ext}"
            );
            assert!(
                all.iter()
                    .any(|e| e.key == format!(r"Software\Classes\.{ext}\OpenWithProgids"))
            );
        }
    }
}
