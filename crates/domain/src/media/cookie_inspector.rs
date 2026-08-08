#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CookieVerdict {
    NotApplicable,
    Live,
    Expired,
}

pub trait CookieInspector: Send + Sync {
    fn verdict_for(&self, host: &str) -> CookieVerdict;
}

pub fn check_host_cookies(contents: &str, host: &str, now_epoch_secs: i64) -> CookieVerdict {
    let host = host.trim().to_ascii_lowercase();
    if host.is_empty() {
        return CookieVerdict::NotApplicable;
    }
    let mut matched = false;
    let mut any_live = false;
    for line in contents.lines() {
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < 7 {
            continue;
        }
        let domain = fields[0]
            .strip_prefix("#HttpOnly_")
            .unwrap_or(fields[0])
            .trim_start_matches('.')
            .to_ascii_lowercase();
        if domain.is_empty() {
            continue;
        }
        let applies = host == domain || host.ends_with(&format!(".{domain}"));
        if !applies {
            continue;
        }
        let Ok(expiry) = fields[4].trim().parse::<i64>() else {
            continue;
        };
        if expiry <= 0 {
            continue;
        }
        matched = true;
        if expiry > now_epoch_secs {
            any_live = true;
        }
    }
    match (matched, any_live) {
        (false, _) => CookieVerdict::NotApplicable,
        (true, true) => CookieVerdict::Live,
        (true, false) => CookieVerdict::Expired,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_000_000;

    fn line(domain: &str, expiry: i64, name: &str) -> String {
        format!("{domain}\tTRUE\t/\tTRUE\t{expiry}\t{name}\tvalue")
    }

    #[test]
    fn blank_host_is_not_applicable() {
        let jar = line(".bilibili.tv", NOW + 10, "SESSDATA");
        assert_eq!(
            check_host_cookies(&jar, "   ", NOW),
            CookieVerdict::NotApplicable
        );
    }

    #[test]
    fn no_matching_domain_is_not_applicable() {
        let jar = line(".bilibili.tv", NOW + 10, "SESSDATA");
        assert_eq!(
            check_host_cookies(&jar, "youtube.com", NOW),
            CookieVerdict::NotApplicable
        );
    }

    #[test]
    fn matching_and_unexpired_is_live() {
        let jar = line(".nebula.tv", NOW + 10, "token");
        assert_eq!(
            check_host_cookies(&jar, "nebula.tv", NOW),
            CookieVerdict::Live
        );
    }

    #[test]
    fn subdomain_host_matches_dotted_domain() {
        let jar = line(".bilibili.tv", NOW + 10, "SESSDATA");
        assert_eq!(
            check_host_cookies(&jar, "www.bilibili.tv", NOW),
            CookieVerdict::Live
        );
    }

    #[test]
    fn matching_but_all_expired_is_expired() {
        let jar = line(".nebula.tv", NOW - 10, "token");
        assert_eq!(
            check_host_cookies(&jar, "nebula.tv", NOW),
            CookieVerdict::Expired
        );
    }

    #[test]
    fn one_live_cookie_keeps_the_host_live() {
        let jar = format!(
            "{}\n{}",
            line(".nebula.tv", NOW - 10, "old"),
            line(".nebula.tv", NOW + 10, "fresh")
        );
        assert_eq!(
            check_host_cookies(&jar, "nebula.tv", NOW),
            CookieVerdict::Live
        );
    }

    #[test]
    fn http_only_prefixed_lines_are_parsed() {
        let jar = line("#HttpOnly_.nebula.tv", NOW - 10, "token");
        assert_eq!(
            check_host_cookies(&jar, "nebula.tv", NOW),
            CookieVerdict::Expired
        );
    }

    #[test]
    fn session_only_cookies_are_not_applicable() {
        let jar = line(".nebula.tv", 0, "session");
        assert_eq!(
            check_host_cookies(&jar, "nebula.tv", NOW),
            CookieVerdict::NotApplicable
        );
    }

    #[test]
    fn comments_short_lines_and_junk_expiry_are_ignored() {
        let jar = format!(
            "# Netscape HTTP Cookie File\n\n{}\n{}\n{}",
            line(".", NOW + 10, "empty-domain"),
            ".nebula.tv\tTRUE\t/\tTRUE\tnotanumber\tname\tvalue",
            line(".nebula.tv", NOW + 10, "good")
        );
        assert_eq!(
            check_host_cookies(&jar, "nebula.tv", NOW),
            CookieVerdict::Live
        );
        assert_eq!(
            check_host_cookies(&jar, "whatever.com", NOW),
            CookieVerdict::NotApplicable
        );
    }
}
