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

#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn preserves_total_and_bounds(
            all in prop::collection::vec(any::<i32>(), 0..64),
            offset in 0u32..128,
            limit in 0u32..128,
        ) {
            let page = paginate(&all, PageRequest { offset, limit });
            prop_assert_eq!(page.total, all.len() as u64);
            prop_assert_eq!(page.offset, offset);
            prop_assert_eq!(page.limit, limit);
            prop_assert!(page.items.len() as u64 <= u64::from(limit));
            prop_assert!(page.items.len() <= all.len());
        }

        #[test]
        fn items_are_the_requested_slice(
            all in prop::collection::vec(any::<i32>(), 0..64),
            offset in 0u32..128,
            limit in 0u32..128,
        ) {
            let page = paginate(&all, PageRequest { offset, limit });
            let start = (offset as usize).min(all.len());
            let end = start.saturating_add(limit as usize).min(all.len());
            prop_assert_eq!(page.items, all[start..end].to_vec());
        }

        #[test]
        fn offset_past_end_is_empty(
            all in prop::collection::vec(any::<i32>(), 0..64),
            extra in 0u32..64,
            limit in 0u32..128,
        ) {
            let offset = all.len() as u32 + extra;
            let page = paginate(&all, PageRequest { offset, limit });
            prop_assert!(page.items.is_empty());
            prop_assert_eq!(page.total, all.len() as u64);
        }

        #[test]
        fn consecutive_pages_reassemble_source(
            all in prop::collection::vec(any::<i32>(), 0..64),
            limit in 1u32..16,
        ) {
            let mut collected = Vec::new();
            let mut offset = 0u32;
            loop {
                let page = paginate(&all, PageRequest { offset, limit });
                if page.items.is_empty() {
                    break;
                }
                collected.extend(page.items);
                offset = offset.saturating_add(limit);
            }
            prop_assert_eq!(collected, all);
        }
    }
}
