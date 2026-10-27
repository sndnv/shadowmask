pub const WINDOW_CHUNKS: usize = 8;

pub fn generate_in_windows<E>(
    samples: &[f32],
    chunk_samples: usize,
    mut generate: impl FnMut(&[f32]) -> Result<Vec<String>, E>,
) -> Result<Vec<String>, E> {
    let mut outputs = Vec::new();
    for window in samples.chunks(WINDOW_CHUNKS * chunk_samples) {
        outputs.extend(generate(window)?);
    }
    Ok(outputs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_hold_whole_chunks_except_the_last() {
        let samples = vec![0.0; 2 * WINDOW_CHUNKS * 10 + 15];
        let mut lengths = Vec::new();

        generate_in_windows(&samples, 10, |window| {
            lengths.push(window.len());
            Ok::<_, String>(Vec::new())
        })
        .unwrap();

        assert_eq!(lengths, vec![WINDOW_CHUNKS * 10, WINDOW_CHUNKS * 10, 15]);
    }

    #[test]
    fn outputs_are_joined_in_window_order() {
        let samples: Vec<f32> = (0..3 * WINDOW_CHUNKS).map(|i| i as f32).collect();

        let outputs = generate_in_windows(&samples, 1, |window| {
            Ok::<_, String>(window.iter().map(|sample| sample.to_string()).collect())
        })
        .unwrap();

        let expected: Vec<String> = (0..3 * WINDOW_CHUNKS).map(|i| i.to_string()).collect();
        assert_eq!(outputs, expected);
    }

    #[test]
    fn the_first_failure_stops_the_rest() {
        let samples = vec![0.0; 3 * WINDOW_CHUNKS];
        let mut calls = 0;

        let result = generate_in_windows(&samples, 1, |_| {
            calls += 1;
            if calls == 2 { Err("out of memory".to_owned()) } else { Ok(vec!["text".to_owned()]) }
        });

        assert_eq!(result, Err("out of memory".to_owned()));
        assert_eq!(calls, 2);
    }

    #[test]
    fn no_samples_means_no_calls() {
        let mut calls = 0;

        let outputs = generate_in_windows(&[], 10, |_| {
            calls += 1;
            Ok::<_, String>(vec!["text".to_owned()])
        })
        .unwrap();

        assert!(outputs.is_empty());
        assert_eq!(calls, 0);
    }
}
