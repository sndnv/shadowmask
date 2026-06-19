use domain::common::{Page, PageRequest};

pub(crate) fn paginate<T: Clone>(all: &[T], page: PageRequest) -> Page<T> {
    let total = all.len() as u64;
    let start = (page.offset as usize).min(all.len());
    let end = start.saturating_add(page.limit as usize).min(all.len());
    Page {
        items: all[start..end].to_vec(),
        total,
        offset: page.offset,
        limit: page.limit,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slices_within_bounds() {
        let all = vec![1, 2, 3, 4, 5];
        let page = paginate(
            &all,
            PageRequest {
                offset: 1,
                limit: 2,
            },
        );
        assert_eq!(page.items, vec![2, 3]);
        assert_eq!(page.total, 5);
        assert_eq!(page.offset, 1);
        assert_eq!(page.limit, 2);
    }

    #[test]
    fn clamps_offset_and_limit_past_end() {
        let all = vec![1, 2, 3];
        let page = paginate(
            &all,
            PageRequest {
                offset: 5,
                limit: 10,
            },
        );
        assert!(page.items.is_empty());
        assert_eq!(page.total, 3);

        let page = paginate(
            &all,
            PageRequest {
                offset: 1,
                limit: 10,
            },
        );
        assert_eq!(page.items, vec![2, 3]);
    }

    #[test]
    fn empty_source_yields_empty_page() {
        let all: Vec<i32> = Vec::new();
        let page = paginate(
            &all,
            PageRequest {
                offset: 0,
                limit: 5,
            },
        );
        assert!(page.items.is_empty());
        assert_eq!(page.total, 0);
    }
}
