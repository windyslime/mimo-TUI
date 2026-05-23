use std::collections::HashMap;
use std::path::PathBuf;

pub const DEFAULT_K1: f64 = 1.5;
pub const DEFAULT_B: f64 = 0.75;

pub struct RecallArchive {
    pub archive_dir: PathBuf,
    pub k1: f64,
    pub b: f64,
}

#[derive(Debug, Clone)]
pub struct RecallHit {
    pub session_id: String,
    pub message_index: usize,
    pub role: String,
    pub score: f64,
    pub excerpt: String,
}

impl RecallArchive {
    pub fn new(archive_dir: PathBuf) -> Self {
        Self {
            archive_dir,
            k1: DEFAULT_K1,
            b: DEFAULT_B,
        }
    }

    pub fn search(
        &self,
        query: &str,
        session_content: &str,
        max_results: usize,
        session_id: &str,
    ) -> Vec<RecallHit> {
        let query_tokens = Self::tokenize(query);
        if query_tokens.is_empty() {
            return Vec::new();
        }

        let messages: Vec<&str> = session_content.lines().collect();

        let mut hits: Vec<(usize, &str, f64)> = Vec::new();

        let avg_len = if messages.is_empty() {
            1
        } else {
            messages.iter().map(|m| m.len()).sum::<usize>() / messages.len()
        };

        let doc_freqs = Self::compute_doc_frequency(&query_tokens, &messages);

        let n = messages.len() as f64;

        for (idx, message) in messages.iter().enumerate() {
            let score = Self::bm25_score(
                &query_tokens,
                message,
                message.len(),
                avg_len,
                &doc_freqs,
                n,
                self.k1,
                self.b,
            );

            if score > 0.0 {
                hits.push((idx, message, score));
            }
        }

        hits.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

        hits.into_iter()
            .take(max_results)
            .map(|(idx, content, score)| RecallHit {
                session_id: session_id.to_string(),
                message_index: idx,
                role: "assistant".to_string(),
                score,
                excerpt: Self::best_window(content, &query_tokens, 240),
            })
            .collect()
    }

    fn tokenize(text: &str) -> Vec<String> {
        text.to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|t| t.len() > 1)
            .map(String::from)
            .collect()
    }

    fn compute_doc_frequency<'a>(
        query_tokens: &[String],
        messages: &[&'a str],
    ) -> HashMap<String, usize> {
        let mut doc_freqs = HashMap::new();
        for token in query_tokens {
            let mut count = 0;
            for message in messages {
                let lower = message.to_lowercase();
                if lower.contains(token) {
                    count += 1;
                }
            }
            doc_freqs.insert(token.clone(), count);
        }
        doc_freqs
    }

    fn bm25_score(
        query_tokens: &[String],
        doc: &str,
        doc_len: usize,
        avg_doc_len: usize,
        doc_freqs: &HashMap<String, usize>,
        n: f64,
        k1: f64,
        b: f64,
    ) -> f64 {
        let doc_lower = doc.to_lowercase();
        let mut score = 0.0;

        for token in query_tokens {
            let df = doc_freqs.get(token).copied().unwrap_or(0);
            if df == 0 {
                continue;
            }

            let idf = ((n - df as f64 + 0.5) / (df as f64 + 0.5) + 1.0).ln();

            let term_freq = doc_lower.matches(token.as_str()).count() as f64;
            let norm_doc_len = doc_len as f64 / avg_doc_len as f64;
            let tf = (term_freq * (k1 + 1.0)) / (term_freq + k1 * (1.0 - b + b * norm_doc_len));

            score += idf * tf;
        }

        score
    }

    fn best_window(text: &str, query_tokens: &[String], window_size: usize) -> String {
        if text.len() <= window_size {
            return Self::align_char_boundary(text, 0, text.len()).to_string();
        }

        let text_lower = text.to_lowercase();

        let mut best_start = 0;
        let mut best_score = 0;

        let max_start = text.len().saturating_sub(window_size);

        for start in (0..=max_start).step_by(4) {
            let end = Self::align_char_boundary(text, start + window_size, text.len());
            let window = &text_lower[start..end.min(text_lower.len())];

            let mut score = 0;
            for token in query_tokens {
                score += window.matches(token.as_str()).count();
            }

            if score > best_score {
                best_score = score;
                best_start = start;
            }
        }

        let actual_end = Self::align_char_boundary(text, best_start + window_size, text.len());
        let excerpt = &text[best_start..actual_end];

        let truncated_start = if best_start > 0 { "..." } else { "" };
        let truncated_end = if actual_end < text.len() { "..." } else { "" };

        format!("{}{}{}", truncated_start, excerpt, truncated_end)
    }

    fn align_char_boundary(text: &str, pos: usize, max: usize) -> usize {
        let pos = pos.min(text.len()).min(max);
        if pos >= text.len() {
            return text.len();
        }

        let mut current = pos;
        while current < text.len() && !text.is_char_boundary(current) {
            current += 1;
        }
        current.min(max)
    }
}
