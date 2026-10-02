use crate::media::FORMAT_TOKENS;

pub fn same_release(video_name: &str, release_name: &str) -> bool {
    match (release_tag(video_name), release_tag(release_name)) {
        (Some(video), Some(release)) => video == release,
        _ => false,
    }
}

pub fn release_tag(name: &str) -> Option<String> {
    let basename = name.rsplit(['/', '\\']).next().unwrap_or(name).to_lowercase();
    let hyphen = last_unbracketed_hyphen(&basename)?;
    let (before, after) = (&basename[..hyphen], &basename[hyphen + 1..]);
    let tag: String = after.chars().take_while(char::is_ascii_alphanumeric).collect();
    let rest = &after[tag.len()..];
    let leading = words(before);
    let plausible = tag.len() >= 2
        && tag.len() <= 16
        && tag.chars().any(|c| c.is_ascii_alphabetic())
        && !is_format_token(&tag)
        && leading.len() >= 2
        && leading.last().is_some_and(|word| is_format_token(word) || is_number(word))
        && names_a_format(before)
        && !names_a_format(rest);
    plausible.then_some(tag)
}

fn last_unbracketed_hyphen(text: &str) -> Option<usize> {
    let mut depth = 0usize;
    let mut found = None;
    for (index, c) in text.char_indices() {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            '-' if depth == 0 => found = Some(index),
            _ => {}
        }
    }
    found
}

fn words(text: &str) -> Vec<&str> {
    text.split(|c: char| !c.is_ascii_alphanumeric()).filter(|word| !word.is_empty()).collect()
}

fn names_a_format(text: &str) -> bool {
    let spoken = format!(" {} ", words(text).join(" "));
    FORMAT_TOKENS
        .iter()
        .filter(|token| token.truncates_a_title)
        .any(|token| spoken.contains(&format!(" {} ", words(token.text).join(" "))))
}

fn is_number(word: &str) -> bool {
    word.chars().all(|c| c.is_ascii_digit())
}

fn is_format_token(word: &str) -> bool {
    FORMAT_TOKENS.iter().any(|token| token.text == word)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHAPES: &[(&str, Option<&str>)] = &[
        ("Neon.Harbor.2017.1080p.BluRay.x264-TAG", Some("tag")),
        ("Neon.Harbor.2017.720p.BluRay.x264-TAG", Some("tag")),
        ("neon.harbor.2017.2160p.uhd.bluray.x265-other", Some("other")),
        ("Cold Cases 2004 DVDRip XviD AC3-TAG", Some("tag")),
        ("cold.cases.2004.dvdrip.xvid.ac3-tag", Some("tag")),
        ("Paper.Skies.1999.1080p.BluRay.x264-TAG[note]", Some("tag")),
        ("Paper.Skies.1999.1080p.BluRay.x264-TAG.mkv", Some("tag")),
        ("/media/Cold-Cases/Paper.Skies.1999.1080p.BluRay.x264-TAG.mkv", Some("tag")),
        ("Film.2019.1080p.WEB-DL.DD5.1.H264-TAG.mkv", Some("tag")),
        ("Film.2019.1080p.WEB-DL.DD5.1.H264-TAG", Some("tag")),
        ("Sun-Down 2002 1080p BluRay x264-TAG.mkv", Some("tag")),
        ("Sun Down 2004 1080p XviD AC3-TAG", Some("tag")),
        ("Neon Harbor 2017 1080p BluRay x264-TAG HI", Some("tag")),
        ("Neon.Harbor.2017.1080p.BluRay.x264-TAG HI.srt", Some("tag")),
        ("Movie.2019.Blu-Ray-TAG.mkv", Some("tag")),
        ("Neon.Harbor.2017.1080p.WEB-DL.DDP5.1.H.264-TAG", Some("tag")),
        ("Neon.Harbor.2017.1080p.WEB-DL.DDP5.1.H.264-TAG.mkv", Some("tag")),
        ("Neon.Harbor.2017.WEB.DDP5.1.H.264-TAG", Some("tag")),
        ("Neon.Harbor.2017.1080p.WEB-DL.DD5.1-TAG", Some("tag")),
        ("Paper.Skies.1999.1080p.BluRay.REMUX.AVC.DTS-HD.MA.5.1-TAG", Some("tag")),
        ("Paper Skies (1999) [Bluray-1080p][DTS-HD MA 5.1]-TAG", Some("tag")),
        ("Neon Harbor - S01E01 - Pilot [WEBDL-1080p][EAC3 5.1][h264]-TAG.mkv", Some("tag")),
        ("Neon Harbor 2017 1080p WEB-DL DDP5 1 H 264-TAG", Some("tag")),
        ("Neon Harbor 2017 WEB DDP5 1 H 264-TAG", Some("tag")),
        ("Neon.Harbor.2017.1080p.BluRay.x264-TAG.en.srt", Some("tag")),
        ("Neon.Harbor.2017.1080p.BluRay.x264-TAG.eng.forced.srt", Some("tag")),
        ("Neon.Harbor.2017.1080p.BluRay.x264-TAG (Re-Sync)", Some("tag")),
        ("Sun-Down.2002.1080p.BluRay.x264.mkv", None),
        ("Rain-Down.2015.1080p.BluRay.x264.mkv", None),
        ("Cold-Cases.S01E01.1080p.mkv", None),
        ("Paper.Skies.1999.2160p.UHD.BluRay.REMUX.DTS-HD.MA.5.1.mkv", None),
        ("Static.Bloom.2010.2160p.UHD.BluRay.REMUX.DTS-HD.MA.5.1.mkv", None),
        ("Movie.2019.Blu-Ray.x264.mkv", None),
        ("Movie.2019.10-bit.1080p.mkv", None),
        ("Movie.2019.1080p.10-bit.mkv", None),
        ("Sun-Down (2002).mkv", None),
        ("Rain-Down (2015).mkv", None),
        ("Sun-Down (2002)[note].mkv", None),
        ("Sun-Down[2002][1080p].mkv", None),
        ("Rain-Down[2015][1080p].mkv", None),
        ("Sun-Down{2002}{1080p}.mkv", None),
        ("/m/Sun-Down (2002)/Sun-Down (2002).mkv", None),
        ("Sun-Down 2002 1080p x264", None),
        ("Film.2019.1080p.WEB-DL", None),
        ("Film.2019.BluRay-1080p", None),
        ("Film.2019.BluRay-x264", None),
        ("Film.2019-hevc", None),
        ("Film.2019.1080p.E-AC3.mkv", None),
        ("Film.2019.1080p.BluRay.x264-AAC.mkv", None),
        ("Film.2019.1080p.x264-EAC3.mkv", None),
        ("Film.2019.x264.DVD-Rip.mkv", None),
        ("Film.2019.x264.WEB-Rip.mkv", None),
        ("Film.2019.1080p.BluRay.x264-UNRATED.mkv", None),
        ("Film.2019.1080p.BluRay.x264-IMAX.mkv", None),
        ("Film.2019.1080p.BluRay.x264-DC.mkv", None),
        ("Film.2019.HDRip.XviD-TVRIP.mkv", None),
        ("/media/Blu-Ray/movie.avi", None),
        ("/m/Paper Skies (1999)/Paper Skies (1999).mkv", None),
        ("Neon.Harbor-TAG.mkv", None),
        ("Rip-Neon (1971).mkv", None),
        ("The Rip-Neon (1971).mkv", None),
        ("Multi-Paper (1995).mkv", None),
        ("The Multi-Paper (1995).mkv", None),
        ("Neon.Harbor.2017.1080p.BluRay.x264-TAG.pt-BR.srt", None),
        ("Neon Harbor - 01 [1080p][Dual-Audio].mkv", None),
        ("Neon Harbor - 01 [1080p][Multi-Sub].mkv", None),
        ("Neon Harbor (2017) [Bluray-1080p] {edition-Directors Cut}.mkv", None),
        ("Neon.Harbor.1982.1080p.BluRay.x264.Directors-Cut.mkv", None),
        ("Neon.Harbor.2017.1080p.BluRay.DTS-HDMA.x264.mkv", None),
        ("Neon Harbor - 01 (BD 1080p x264-Hi10P AAC).mkv", None),
        ("Neon.Harbor.1999.DVDRip.XviD-TAG-CD1.avi", None),
        ("Neon.Harbor.2017.1080p.BluRay.x264-TAG-Extra.mkv", None),
        ("Neon Harbor - S01E01 - HDTV-720p - Long-Night.mkv", None),
        ("Neon.Harbor.2017.1080p.BluRay.x264-TAG Non-HI", None),
        ("Neon Harbor (2017) 1080p BluRay x264 [imdbid-tt0000001].mkv", None),
        ("Remux-Lantern (2015).mkv", None),
        ("Neon Harbor 1999-2004 1080p BluRay x264", None),
        ("Neon.Harbor.S01-S03.1080p.BluRay.x264.mkv", None),
        ("Neon Harbor - 01 (1080p) [ABCD1234].mkv", None),
        ("Neon.Harbor.2017.1080p.BluRay.x264.[TAG].mkv", None),
        ("tag-lantern103.avi", None),
        ("", None),
        ("-", None),
        ("a-b", None),
        ("....", None),
        ("movie", None),
    ];

    #[test]
    fn every_shape_the_tag_rule_reads_and_every_shape_it_refuses() {
        for (name, expected) in SHAPES {
            assert_eq!(
                release_tag(name).as_deref(),
                *expected,
                "[{name}] is the shape that disagrees"
            );
        }
    }

    #[test]
    fn no_two_shapes_without_a_tag_are_ever_called_the_same_release() {
        let untagged: Vec<&str> =
            SHAPES.iter().filter(|(_, tag)| tag.is_none()).map(|(name, _)| *name).collect();
        for left in &untagged {
            for right in &untagged {
                assert!(!same_release(left, right), "[{left}] matched [{right}]");
            }
        }
    }

    #[test]
    fn one_release_matches_itself() {
        let name = "Neon.Harbor.2017.1080p.BluRay.x264-TAG";
        assert!(same_release(name, name));
    }

    #[test]
    fn the_same_tag_at_another_resolution_still_matches() {
        assert!(same_release(
            "Neon.Harbor.2017.1080p.BluRay.x264-TAG",
            "Neon.Harbor.2017.720p.BluRay.x264-TAG",
        ));
    }

    #[test]
    fn a_different_tag_never_matches() {
        assert!(!same_release(
            "Neon.Harbor.2017.1080p.BluRay.x264-TAG",
            "neon.harbor.2017.2160p.uhd.bluray.x265-other",
        ));
        assert!(!same_release(
            "Some.Film.2019.1080p.BluRay.x264-AAA",
            "Some.Film.2019.1080p.BluRay.x264-BBB",
        ));
    }

    #[test]
    fn a_release_name_that_is_only_a_title_never_matches() {
        assert!(!same_release("Neon.Harbor.2017.1080p.BluRay.x264-TAG", "Neon Harbor",));
    }

    #[test]
    fn scoring_is_case_and_separator_blind() {
        assert!(same_release(
            "Cold Cases 2004 DVDRip XviD AC3-TAG",
            "cold.cases.2004.dvdrip.xvid.ac3-tag",
        ));
    }

    #[test]
    fn a_file_with_no_tag_never_matches_one_that_shares_its_name() {
        assert!(!same_release(
            "Sun-Down.2002.1080p.BluRay.x264.mkv",
            "Sun-Down.2002.1080p.BluRay.x264-TAG",
        ));
        assert!(!same_release("Sun-Down (2002).mkv", "Sun-Down 2002 1080p x264"));
    }

    #[test]
    fn a_file_with_no_tag_is_never_a_match_in_either_direction() {
        for (name, _) in SHAPES.iter().filter(|(_, tag)| tag.is_none()) {
            assert!(!same_release("Film.2019.1080p.BluRay.x264-AAA", name), "[{name}]");
            assert!(!same_release(name, "Film.2019.1080p.BluRay.x264-AAA"), "[{name}]");
        }
    }
}
