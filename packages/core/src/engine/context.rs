const DEFAULT_CONTEXT_WINDOW: usize = 1_000_000;
const DEFAULT_COMPRESSION_THRESHOLD: f64 = 0.8;
const MEMORY_PRESERVE_RATIO: f64 = 0.1;

pub struct ContextManager {
    max_tokens: usize,
    compression_threshold: f64,
    current_token_count: usize,
    memory_token_count: usize,
}

pub struct CompressionResult {
    pub tokens_to_trim: usize,
    pub preserve_recent: usize,
    pub memory_preserved: bool,
}

impl ContextManager {
    pub fn new() -> Self {
        Self {
            max_tokens: DEFAULT_CONTEXT_WINDOW,
            compression_threshold: DEFAULT_COMPRESSION_THRESHOLD,
            current_token_count: 0,
            memory_token_count: 0,
        }
    }

    pub fn with_limits(max_tokens: usize, compression_threshold: f64) -> Self {
        Self {
            max_tokens,
            compression_threshold,
            current_token_count: 0,
            memory_token_count: 0,
        }
    }

    pub fn update_token_count(&mut self, count: usize) {
        self.current_token_count = count;
    }

    pub fn update_memory_token_count(&mut self, count: usize) {
        self.memory_token_count = count;
    }

    pub fn should_compress(&self) -> bool {
        let ratio = self.current_token_count as f64 / self.max_tokens as f64;
        ratio >= self.compression_threshold
    }

    pub fn compute_compression(&self, message_count: usize) -> CompressionResult {
        if !self.should_compress() {
            return CompressionResult {
                tokens_to_trim: 0,
                preserve_recent: message_count,
                memory_preserved: false,
            };
        }

        let target_tokens = (self.max_tokens as f64 * 0.5) as usize;
        let tokens_to_trim = self.current_token_count.saturating_sub(target_tokens);

        let preserve_recent = (message_count as f64 * 0.3).ceil() as usize;

        let memory_budget = (self.max_tokens as f64 * MEMORY_PRESERVE_RATIO) as usize;
        let memory_preserved = self.memory_token_count <= memory_budget;

        CompressionResult {
            tokens_to_trim,
            preserve_recent,
            memory_preserved,
        }
    }

    pub fn max_tokens(&self) -> usize {
        self.max_tokens
    }

    pub fn current_tokens(&self) -> usize {
        self.current_token_count
    }

    pub fn memory_tokens(&self) -> usize {
        self.memory_token_count
    }

    pub fn remaining_tokens(&self) -> usize {
        self.max_tokens.saturating_sub(self.current_token_count)
    }
}

impl Default for ContextManager {
    fn default() -> Self {
        Self::new()
    }
}
