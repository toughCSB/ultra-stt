//! Live typing: decide which recognised words are stable enough to type.
//!
//! Each pass re-decodes the whole current utterance. A word is committed once two
//! consecutive passes agree on it, so text typed into the target app rarely needs
//! correction. Committed words are never retracted; later passes are aligned to them
//! by position.

/// Whole-window outputs Whisper is known to invent for noise or near-silence.
const HALLUCINATIONS: &[&str] = &[
    "감사합니다",
    "시청해주셔서 감사합니다",
    "시청해 주셔서 감사합니다",
    "구독과 좋아요 부탁드립니다",
    "다음 영상에서 만나요",
    "MBC 뉴스",
];

const TRAILING_PUNCTUATION: [char; 6] = ['.', ',', '?', '!', '…', '。'];

fn words(text: &str) -> Vec<String> {
    text.split_whitespace().map(str::to_owned).collect()
}

/// Words compare equal when they differ only by trailing punctuation.
fn same_word(left: &str, right: &str) -> bool {
    left.trim_end_matches(TRAILING_PUNCTUATION) == right.trim_end_matches(TRAILING_PUNCTUATION)
}

/// True when the whole text is a known hallucination (or empty after cleanup).
pub fn is_hallucination(text: &str) -> bool {
    let cleaned: String = text
        .chars()
        .filter(|c| !TRAILING_PUNCTUATION.contains(c))
        .collect();
    let normalized = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    normalized.is_empty() || HALLUCINATIONS.contains(&normalized.as_str())
}

fn spoken_chars(words: &[String]) -> usize {
    words.iter().map(|word| word.chars().count()).sum()
}

#[derive(Debug, Default)]
pub struct LiveAgreement {
    committed: Vec<String>,
    /// Uncommitted words of the previous pass, already aligned after `committed`.
    previous_rest: Option<Vec<String>>,
}

impl LiveAgreement {
    /// Index in `current` where the uncommitted part starts. An unchanged prefix aligns
    /// exactly; when Whisper rewrote already-typed words, align by spoken length instead
    /// (Korean revisions such as "다리 및" -> "다리와" keep roughly the same characters).
    fn align(&self, current: &[String]) -> usize {
        let n = self.committed.len();
        if current.len() >= n
            && current
                .iter()
                .zip(&self.committed)
                .all(|(now, typed)| same_word(now, typed))
        {
            return n;
        }
        let target = spoken_chars(&self.committed);
        (0..=current.len())
            .min_by_key(|&k| spoken_chars(&current[..k]).abs_diff(target))
            .unwrap_or(0)
    }

    /// Feed one pass over the current utterance; returns the newly committed words.
    pub fn update(&mut self, hypothesis: &str) -> Vec<String> {
        let current = words(hypothesis);
        let rest = current[self.align(&current)..].to_vec();
        let newly: Vec<String> = match &self.previous_rest {
            Some(previous) => rest
                .iter()
                .zip(previous)
                .take_while(|(now, before)| same_word(now, before))
                .map(|(now, _)| now.clone())
                .collect(),
            None => Vec::new(),
        };
        self.committed.extend(newly.iter().cloned());
        self.previous_rest = Some(rest[newly.len()..].to_vec());
        newly
    }

    /// Final pass for the utterance: returns every word not yet committed and resets.
    pub fn finish(&mut self, hypothesis: &str) -> Vec<String> {
        let current = words(hypothesis);
        let rest = current[self.align(&current)..].to_vec();
        *self = Self::default();
        rest
    }

    /// Uncommitted words of the latest pass, for on-screen preview only.
    pub fn pending(&self) -> Vec<String> {
        self.previous_rest.clone().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_word_is_committed_only_after_two_passes_agree() {
        let mut agreement = LiveAgreement::default();
        assert!(agreement.update("다리 및").is_empty());
        assert_eq!(agreement.update("다리 및 수직"), vec!["다리", "및"]);
        assert_eq!(agreement.update("다리 및 수직 간격은"), vec!["수직"]);
        assert_eq!(agreement.pending(), vec!["간격은"]);
    }

    #[test]
    fn a_changing_tail_is_held_back() {
        let mut agreement = LiveAgreement::default();
        agreement.update("오늘 회의는");
        assert_eq!(agreement.update("오늘 회의를 시작"), vec!["오늘"]);
        assert!(agreement.update("오늘 회의록 시작합니다").is_empty());
        assert_eq!(
            agreement.update("오늘 회의록 시작합니다."),
            vec!["회의록", "시작합니다."]
        );
    }

    #[test]
    fn trailing_punctuation_does_not_block_agreement() {
        let mut agreement = LiveAgreement::default();
        agreement.update("네 알겠습니다.");
        assert_eq!(
            agreement.update("네 알겠습니다 그럼"),
            vec!["네", "알겠습니다"]
        );
    }

    #[test]
    fn a_rewritten_typed_prefix_is_aligned_by_spoken_length() {
        let mut agreement = LiveAgreement::default();
        agreement.update("다리 및");
        assert_eq!(agreement.update("다리 및 수직"), vec!["다리", "및"]);
        // Whisper rewrites already-typed words; typing continues after them, no word lost.
        assert_eq!(agreement.update("다리와 수직 간격"), vec!["수직"]);
        assert_eq!(agreement.update("다리와 수직 간격 15m"), vec!["간격"]);
    }

    #[test]
    fn finish_returns_every_uncommitted_word_and_resets() {
        let mut agreement = LiveAgreement::default();
        agreement.update("안녕하세요 오늘");
        agreement.update("안녕하세요 오늘은");
        assert_eq!(
            agreement.finish("안녕하세요 오늘은 날씨가 좋네요."),
            vec!["오늘은", "날씨가", "좋네요."]
        );
        assert!(agreement.pending().is_empty());
        assert!(agreement.update("새 문장").is_empty());
    }

    #[test]
    fn finish_with_a_shorter_final_returns_nothing() {
        let mut agreement = LiveAgreement::default();
        agreement.update("하나 둘 셋");
        agreement.update("하나 둘 셋 넷");
        assert!(agreement.finish("하나").is_empty());
    }

    #[test]
    fn known_hallucinations_are_detected_only_as_whole_outputs() {
        assert!(is_hallucination(" 감사합니다."));
        assert!(is_hallucination("시청해주셔서 감사합니다!"));
        assert!(is_hallucination(" . "));
        assert!(!is_hallucination("도와주셔서 감사합니다."));
        assert!(!is_hallucination("다리 및 수직 간격"));
    }
}
