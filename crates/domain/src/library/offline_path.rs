pub fn is_offline(path: &str, offline: &[String]) -> bool {
    offline.iter().any(|dir| {
        path.strip_prefix(dir.trim_end_matches('/'))
            .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn offline(paths: &[&str]) -> Vec<String> {
        paths.iter().map(|path| (*path).to_owned()).collect()
    }

    #[test]
    fn a_path_is_offline_at_or_under_an_offline_path() {
        assert!(is_offline("/media/movies", &offline(&["/media/movies"])));
        assert!(is_offline("/media/movies/a.mkv", &offline(&["/media/movies"])));
        assert!(is_offline("/media/movies/a.mkv", &offline(&["/media/movies/"])));
        assert!(is_offline("/media/movies/a.mkv", &offline(&["/tv", "/"])));
    }

    #[test]
    fn a_path_sharing_only_the_first_letters_is_not_offline() {
        assert!(!is_offline("/media/movies2/a.mkv", &offline(&["/media/movies"])));
        assert!(!is_offline("/media/a.mkv", &offline(&["/media/a"])));
        assert!(!is_offline("/media/a.mkv", &[]));
    }
}
