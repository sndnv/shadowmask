use std::collections::BTreeSet;

use crate::catalog::TitleId;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DuplicateCandidateId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DuplicateKind {
    Duplicate,
    MultiPart,
}

#[derive(Debug, Clone)]
pub struct DuplicateCandidate {
    pub id: DuplicateCandidateId,
    pub title: TitleId,
    pub paths: Vec<String>,
}

impl DuplicateCandidate {
    pub fn kind(&self) -> DuplicateKind {
        if is_multi_part(&self.paths) { DuplicateKind::MultiPart } else { DuplicateKind::Duplicate }
    }
}

const DISC_WORDS: [&str; 5] = ["cd", "disc", "disk", "part", "pt"];

fn is_multi_part(paths: &[String]) -> bool {
    if paths.len() < 2 {
        return false;
    }
    let mut shapes = BTreeSet::new();
    let mut parts = BTreeSet::new();
    for path in paths {
        let Some((shape, part)) = disc_marker(basename(path)) else {
            return false;
        };
        shapes.insert(shape);
        parts.insert(part);
    }
    shapes.len() == 1 && parts.len() == paths.len()
}

fn basename(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

fn disc_marker(name: &str) -> Option<(String, u32)> {
    let lower = name.to_lowercase();
    let bytes = lower.as_bytes();
    for word in DISC_WORDS {
        let mut from = 0;
        while let Some(at) = lower[from..].find(word).map(|found| found + from) {
            from = at + word.len();
            if at > 0 && bytes[at - 1].is_ascii_alphanumeric() {
                continue;
            }
            let after = &lower[from..];
            let rest = after.strip_prefix([' ', '.', '_', '-']).unwrap_or(after);
            let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
            if digits.is_empty() || digits.len() > 2 {
                continue;
            }
            let part = digits.parse().expect("one or two ASCII digits always parse");
            let mut shape = String::with_capacity(lower.len());
            shape.push_str(&lower[..at]);
            shape.push('#');
            shape.push_str(&rest[digits.len()..]);
            return Some((shape, part));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::MovieId;

    fn candidate(paths: &[&str]) -> DuplicateCandidate {
        DuplicateCandidate {
            id: DuplicateCandidateId("dup:x".to_owned()),
            title: TitleId::Movie(MovieId("m1".to_owned())),
            paths: paths.iter().map(|p| (*p).to_owned()).collect(),
        }
    }

    #[test]
    fn two_discs_of_one_release_are_one_title_in_parts() {
        let c = candidate(&[
            "/m/Paper.Skies.(2004)/Paper.Skies.2004.DVDRip.XviD.AC3.CD1-TAG.avi",
            "/m/Paper.Skies.(2004)/Paper.Skies.2004.DVDRip.XviD.AC3.CD2-TAG.avi",
        ]);
        assert_eq!(c.kind(), DuplicateKind::MultiPart);
    }

    #[test]
    fn parts_that_disagree_about_case_still_pair_up() {
        let c = candidate(&[
            "/m/Pioneer.One.(2003)/Pioneer.One.2003.CD2.WS.DVDRip.XviD-TAG.avi",
            "/m/Pioneer.One.(2003)/pioneer.one.2003.cd1.ws.dvdrip.xvid-tag.avi",
        ]);
        assert_eq!(c.kind(), DuplicateKind::MultiPart);
    }

    #[test]
    fn parts_in_their_own_folders_still_pair_up() {
        let c = candidate(&[
            "/m/Deep.Field/CD1/deep.field.dvdrip.xvid-tag-cd1.avi",
            "/m/Deep.Field/CD2/deep.field.dvdrip.xvid-tag-cd2.avi",
        ]);
        assert_eq!(c.kind(), DuplicateKind::MultiPart);
    }

    #[test]
    fn two_rips_of_one_film_are_duplicates_not_parts() {
        let c = candidate(&[
            "/m/Film.(1999)/Film.1999.DVDRip.XviD-AAA.avi",
            "/m/Film.(1999)/Film.1999.DVDRip.XviD-BBB.avi",
        ]);
        assert_eq!(c.kind(), DuplicateKind::Duplicate);
    }

    #[test]
    fn a_word_beginning_with_disc_is_not_a_part_marker() {
        let c =
            candidate(&["/tv/Discovery.Notes.One.XviD.avi", "/tv/Discovery.Notes.Two.XviD.avi"]);
        assert_eq!(c.kind(), DuplicateKind::Duplicate);
    }

    #[test]
    fn a_marker_glued_to_a_word_is_not_a_part_marker() {
        let c = candidate(&["/m/abcd1.avi", "/m/abcd2.avi"]);
        assert_eq!(c.kind(), DuplicateKind::Duplicate);
    }

    #[test]
    fn the_same_disc_twice_is_a_duplicate() {
        let c = candidate(&["/m/a/film.cd1.avi", "/m/b/film.cd1.avi"]);
        assert_eq!(c.kind(), DuplicateKind::Duplicate);
    }

    #[test]
    fn parts_of_different_releases_are_not_a_pair() {
        let c = candidate(&["/m/film.cd1-aaa.avi", "/m/film.cd2-bbb.avi"]);
        assert_eq!(c.kind(), DuplicateKind::Duplicate);
    }

    #[test]
    fn three_discs_pair_up() {
        let c = candidate(&["/m/f.cd1.avi", "/m/f.cd2.avi", "/m/f.cd3.avi"]);
        assert_eq!(c.kind(), DuplicateKind::MultiPart);
    }

    #[test]
    fn every_way_a_part_is_written_pairs_up() {
        for (first, second) in [
            ("/m/f.CD 1.avi", "/m/f.CD 2.avi"),
            ("/m/f.Disc-1.avi", "/m/f.Disc-2.avi"),
            ("/m/f.Part1.avi", "/m/f.Part2.avi"),
            ("/m/f.Pt.1.avi", "/m/f.Pt.2.avi"),
            ("/m/f.part_1.avi", "/m/f.part_2.avi"),
            ("/m/f.Part.11.avi", "/m/f.Part.12.avi"),
        ] {
            assert_eq!(candidate(&[first, second]).kind(), DuplicateKind::MultiPart, "[{first}]");
        }
    }

    #[test]
    fn a_word_that_only_starts_like_a_part_marker_is_not_one() {
        for (first, second) in [
            ("/m/Parts.Unknown.1.avi", "/m/Parts.Unknown.2.avi"),
            ("/m/Script.1.avi", "/m/Script.2.avi"),
            ("/m/f.dvdrip.1.avi", "/m/f.dvdrip.2.avi"),
            ("/m/Film.2004.DVD5.x264.mkv", "/m/Film.2004.DVD9.x264.mkv"),
            ("/m/f.part 100.avi", "/m/f.part 101.avi"),
        ] {
            assert_eq!(candidate(&[first, second]).kind(), DuplicateKind::Duplicate, "[{first}]");
        }
    }

    #[test]
    fn a_lone_path_is_never_multi_part() {
        assert_eq!(candidate(&["/m/f.cd1.avi"]).kind(), DuplicateKind::Duplicate);
    }
}
