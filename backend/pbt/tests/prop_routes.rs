use brew_book_core::routes::pattern_matches;
use proptest::prelude::*;

fn segment() -> impl Strategy<Value = String> {
    "[a-z0-9]{1,8}".prop_map(String::from)
}

proptest! {
    #[test]
    fn generated_pattern_matches_the_path_it_describes(
        segments in prop::collection::vec((any::<bool>(), segment()), 1..4),
    ) {
        let mut pattern = String::new();
        let mut path = String::new();
        for (is_parameter, value) in &segments {
            pattern.push('/');
            path.push('/');
            if *is_parameter {
                pattern.push_str(":value");
            } else {
                pattern.push_str(value);
            }
            path.push_str(value);
        }
        prop_assert!(pattern_matches(&pattern, &path));
    }

    #[test]
    fn different_segment_counts_do_not_match(
        segments in prop::collection::vec(segment(), 1..4),
    ) {
        let pattern = format!("/api/{}", segments.join("/"));
        let shorter = format!("/api/{}", segments[..segments.len() - 1].join("/"));
        prop_assert!(!pattern_matches(&pattern, &shorter));
        let longer = format!("/api/{}/extra", segments.join("/"));
        prop_assert!(!pattern_matches(&pattern, &longer));
    }

    #[test]
    fn a_literal_segment_does_not_match_another_value(
        values in prop::collection::vec(segment(), 2..8),
    ) {
        let pattern = format!("/{}", values.join("/"));
        let mut path = values.clone();
        let index = values.len() / 2;
        path[index] = format!("{}-different", values[index]);
        let path = format!("/{}", path.join("/"));
        prop_assert!(!pattern_matches(&pattern, &path));
    }
}
