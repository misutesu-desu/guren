use std::cmp::Ordering;
use std::path::{Path, PathBuf};

const EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "jfif", "jpe", "png", "webp", "gif", "bmp", "tif", "tiff", "ico",
];

pub fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| EXTENSIONS.iter().any(|x| x.eq_ignore_ascii_case(e)))
}

pub fn list_images(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_ok_and(|t| t.is_file()))
        .map(|entry| entry.path())
        .filter(|p| is_image(p))
        .collect();
    sort(&mut files);
    Ok(files)
}

pub fn sort(files: &mut Vec<PathBuf>) {
    let mut keyed: Vec<(String, PathBuf)> = files.drain(..).map(|p| (name(&p), p)).collect();
    keyed.sort_by(|a, b| natural_cmp(&a.0, &b.0));
    files.extend(keyed.into_iter().map(|(_, p)| p));
}

fn name(p: &Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

/// Compares digit runs by value so that "img2" comes before "img10".
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let (mut a, mut b) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (a.peek().copied(), b.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let na = take_number(&mut a);
                let nb = take_number(&mut b);
                let by_value = na
                    .trim_start_matches('0')
                    .len()
                    .cmp(&nb.trim_start_matches('0').len())
                    .then_with(|| na.trim_start_matches('0').cmp(nb.trim_start_matches('0')));
                if by_value != Ordering::Equal {
                    return by_value;
                }
            }
            (Some(x), Some(y)) => {
                if x != y {
                    return x.cmp(&y);
                }
                a.next();
                b.next();
            }
        }
    }
}

fn take_number(it: &mut std::iter::Peekable<std::str::Chars>) -> String {
    let mut s = String::new();
    while let Some(c) = it.peek().copied().filter(char::is_ascii_digit) {
        s.push(c);
        it.next();
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_sort_by_value() {
        let mut names = vec!["img10.jpg", "img2.jpg", "img1.jpg", "IMG3.jpg", "a.png"];
        names.sort_by(|a, b| natural_cmp(&a.to_lowercase(), &b.to_lowercase()));
        assert_eq!(
            names,
            ["a.png", "img1.jpg", "img2.jpg", "IMG3.jpg", "img10.jpg"]
        );
    }

    #[test]
    fn leading_zeros_compare_by_value() {
        assert_eq!(natural_cmp("frame007", "frame7"), Ordering::Equal);
        assert_eq!(natural_cmp("frame007", "frame10"), Ordering::Less);
    }

    #[test]
    fn extension_check_ignores_case() {
        assert!(is_image(Path::new("x/Foto.JPG")));
        assert!(!is_image(Path::new("x/notes.txt")));
    }
}
