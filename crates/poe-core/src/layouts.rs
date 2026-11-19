//! Zone layout images: which of the player's image files show an Area.
//!
//! The app ships no images. Players put their own in the layouts folder,
//! named by Area ID: `1_1_2 1.jpg`, `1_1_2 2.jpg` (one per layout variant),
//! or inside a folder named after the Area ID (`1_1_2/top exit.png`).

/// File types the Overlay can show.
const IMAGE_EXTENSIONS: [&str; 4] = ["jpg", "jpeg", "png", "webp"];

/// One image for an Area.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutImage {
    /// Path relative to the layouts folder, `/`-separated.
    pub path: String,
    /// What tells this variant apart (the rest of the name), may be empty.
    pub label: String,
}

/// The images among `paths` (relative to the layouts folder) that show
/// `area_id`, ordered by label: unlabelled first, then numbers in order
/// (`2` before `10`), then words.
pub fn images_for<'a>(area_id: &str, paths: impl IntoIterator<Item = &'a str>) -> Vec<LayoutImage> {
    let mut out: Vec<LayoutImage> = paths
        .into_iter()
        .filter_map(|p| {
            let p = p.replace('\\', "/");
            let (dir, file) = p.rsplit_once('/').map_or(("", p.as_str()), |(d, f)| (d, f));
            let (stem, ext) = file.rsplit_once('.')?;
            if !IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str()) {
                return None;
            }
            let label = if dir.rsplit('/').next() == Some(area_id) {
                stem.to_owned()
            } else if dir.is_empty() || !dir.contains(area_id) {
                let (id, rest) = stem.split_once(' ').unwrap_or((stem, ""));
                if id != area_id {
                    return None;
                }
                rest.to_owned()
            } else {
                return None;
            };
            Some(LayoutImage {
                path: p.clone(),
                label,
            })
        })
        .collect();
    out.sort_by(|a, b| (natural_key(&a.label), &a.path).cmp(&(natural_key(&b.label), &b.path)));
    out
}

/// Sort key: a leading number compares as a number, before any text.
fn natural_key(s: &str) -> (bool, u64, String) {
    let digits: String = s.chars().take_while(char::is_ascii_digit).collect();
    let rest = s[digits.len()..].to_lowercase();
    match digits.parse() {
        Ok(n) => (false, n, rest),
        Err(_) => (!s.is_empty(), 0, rest),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_by_name_or_folder_in_natural_order() {
        let files = [
            "1_1_2 10.jpg",
            "1_1_2 2.jpg",
            "1_1_2.png",
            "1_1_21 1.jpg",
            "1_1_2 notes.txt",
            "1_1_2/top exit.webp",
            "1_1_3 1.jpg",
            "act1\\1_1_2 3.JPG",
        ];
        let got: Vec<(String, String)> = images_for("1_1_2", files)
            .into_iter()
            .map(|i| (i.path, i.label))
            .collect();
        assert_eq!(
            got,
            [
                ("1_1_2.png".into(), String::new()),
                ("1_1_2 2.jpg".into(), "2".into()),
                ("act1/1_1_2 3.JPG".into(), "3".into()),
                ("1_1_2 10.jpg".into(), "10".into()),
                ("1_1_2/top exit.webp".into(), "top exit".into()),
            ]
        );
    }
}
