use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub const MEMORY_MD_CHAR_LIMIT: usize = 3000;
pub const USER_MD_CHAR_LIMIT: usize = 1500;
pub const ENTRY_DELIMITER: &str = "§";

#[derive(Debug, Clone, Copy)]
pub enum MemoryTarget {
    Memory,
    User,
}

pub struct MemoryUsage {
    pub pct: u32,
    pub chars: usize,
    pub limit_chars: usize,
}

pub struct MemoryOpResult {
    pub ok: bool,
    pub message: String,
    pub target: MemoryTarget,
    pub usage_pct: u32,
    pub usage_chars: usize,
    pub limit_chars: usize,
}

pub struct MemoryManager {
    memory_path: PathBuf,
    user_path: PathBuf,
    snapshot_memory: Option<String>,
    snapshot_user: Option<String>,
}

impl MemoryManager {
    pub fn new(base_dir: &Path) -> Self {
        let memories_dir = base_dir.join("memories");
        let _ = fs::create_dir_all(&memories_dir);
        Self {
            memory_path: memories_dir.join("MEMORY.md"),
            user_path: memories_dir.join("USER.md"),
            snapshot_memory: None,
            snapshot_user: None,
        }
    }

    pub fn take_snapshot(&mut self) {
        self.snapshot_memory = Self::read_file_content(&self.memory_path);
        self.snapshot_user = Self::read_file_content(&self.user_path);
    }

    pub fn get_system_prompt_block(&self) -> Option<String> {
        let mem = self.snapshot_memory.as_deref().unwrap_or("");
        let usr = self.snapshot_user.as_deref().unwrap_or("");

        let mem_entries = split_entries(mem);
        let usr_entries = split_entries(usr);

        if mem_entries.is_empty() && usr_entries.is_empty() {
            return None;
        }

        let mem_chars = mem.chars().count();
        let mem_pct = usage_pct(mem_chars, MEMORY_MD_CHAR_LIMIT);
        let usr_chars = usr.chars().count();
        let usr_pct = usage_pct(usr_chars, USER_MD_CHAR_LIMIT);

        let mut block = String::new();

        let mem_header = format!(
            "MEMORY (your personal notes) [{}% — {}/{} chars]",
            mem_pct, mem_chars, MEMORY_MD_CHAR_LIMIT
        );
        block.push_str(&section_header(&mem_header));
        block.push('\n');
        block.push_str(&mem_entries.join(ENTRY_DELIMITER));
        block.push('\n');

        let usr_header = format!(
            "USER PROFILE [{}% — {}/{} chars]",
            usr_pct, usr_chars, USER_MD_CHAR_LIMIT
        );
        block.push_str(&section_header(&usr_header));
        block.push('\n');
        block.push_str(&usr_entries.join(ENTRY_DELIMITER));
        block.push('\n');

        Some(block)
    }

    pub fn add(&self, target: MemoryTarget, content: &str) -> MemoryOpResult {
        let cleaned = content.trim();
        if cleaned.is_empty() {
            return MemoryOpResult {
                ok: false,
                message: "Entry content must not be empty.".to_string(),
                target: self.target_label(target),
                usage_pct: 0,
                usage_chars: 0,
                limit_chars: self.limit_for(target),
            };
        }

        let path = self.path_for(target);
        let limit = self.limit_for(target);
        let existing = Self::read_file_content(path).unwrap_or_default();
        let existing_chars = existing.chars().count();
        let new_chars = cleaned.chars().count();

        let needed_chars = if existing_chars == 0 {
            new_chars
        } else {
            existing_chars + ENTRY_DELIMITER.chars().count() + new_chars
        };

        if needed_chars > limit {
            let shortage = needed_chars - limit;
            return MemoryOpResult {
                ok: false,
                message: format!(
                    "Memory full: {}/{} chars used. Need {} more chars.",
                    existing_chars, limit, shortage
                ),
                target: self.target_label(target),
                usage_pct: usage_pct(existing_chars, limit),
                usage_chars: existing_chars,
                limit_chars: limit,
            };
        }

        let new_content = if existing_chars == 0 {
            cleaned.to_string()
        } else {
            format!("{}{}{}", existing, ENTRY_DELIMITER, cleaned)
        };

        match Self::atomic_write(path, &new_content) {
            Ok(()) => {
                let final_chars = new_content.chars().count();
                MemoryOpResult {
                    ok: true,
                    message: format!(
                        "Memory added to {}: \"{}\"",
                        self.target_name(target),
                        cleaned
                    ),
                    target: self.target_label(target),
                    usage_pct: usage_pct(final_chars, limit),
                    usage_chars: final_chars,
                    limit_chars: limit,
                }
            }
            Err(e) => MemoryOpResult {
                ok: false,
                message: format!("Failed to write memory: {}", e),
                target: self.target_label(target),
                usage_pct: usage_pct(existing_chars, limit),
                usage_chars: existing_chars,
                limit_chars: limit,
            },
        }
    }

    pub fn replace(
        &self,
        target: MemoryTarget,
        old_text: &str,
        new_content: &str,
    ) -> MemoryOpResult {
        let path = self.path_for(target);
        let limit = self.limit_for(target);
        let old_text = old_text.trim();
        let new_content = new_content.trim();

        if old_text.is_empty() {
            return MemoryOpResult {
                ok: false,
                message: "old_text must not be empty.".to_string(),
                target: self.target_label(target),
                usage_pct: 0,
                usage_chars: 0,
                limit_chars: limit,
            };
        }

        if new_content.is_empty() {
            return MemoryOpResult {
                ok: false,
                message: "new content must not be empty.".to_string(),
                target: self.target_label(target),
                usage_pct: 0,
                usage_chars: 0,
                limit_chars: limit,
            };
        }

        let existing = Self::read_file_content(path).unwrap_or_default();
        let entries = split_entries(&existing);
        let current_chars = existing.chars().count();

        let matches: Vec<(usize, &str)> = entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.contains(old_text))
            .map(|(i, e)| (i, *e))
            .collect();

        if matches.is_empty() {
            return MemoryOpResult {
                ok: false,
                message: format!(
                    "No entry in {} contains \"{}\".",
                    self.target_name(target),
                    old_text
                ),
                target: self.target_label(target),
                usage_pct: usage_pct(current_chars, limit),
                usage_chars: current_chars,
                limit_chars: limit,
            };
        }

        if matches.len() > 1 {
            let matched_snippets: Vec<String> = matches
                .iter()
                .map(|(_, e)| format!("  - \"{}\"", e))
                .collect();
            return MemoryOpResult {
                ok: false,
                message: format!(
                    "\"{}\" matches {} entries in {}. Provide a more specific old_text:\n{}",
                    old_text,
                    matches.len(),
                    self.target_name(target),
                    matched_snippets.join("\n")
                ),
                target: self.target_label(target),
                usage_pct: usage_pct(current_chars, limit),
                usage_chars: current_chars,
                limit_chars: limit,
            };
        }

        let (idx, _old_entry) = matches[0];
        let mut new_entries: Vec<&str> = entries.iter().copied().collect();
        new_entries[idx] = new_content;

        let new_file_content = new_entries.join(ENTRY_DELIMITER);
        let new_chars = new_file_content.chars().count();

        if new_chars > limit {
            let shortage = new_chars - limit;
            return MemoryOpResult {
                ok: false,
                message: format!(
                    "Memory full: replacement would use {}/{} chars. Need {} more chars.",
                    new_chars, limit, shortage
                ),
                target: self.target_label(target),
                usage_pct: usage_pct(current_chars, limit),
                usage_chars: current_chars,
                limit_chars: limit,
            };
        }

        match Self::atomic_write(path, &new_file_content) {
            Ok(()) => MemoryOpResult {
                ok: true,
                message: format!(
                    "Replaced entry in {}: \"{}\" → \"{}\"",
                    self.target_name(target),
                    old_text,
                    new_content
                ),
                target: self.target_label(target),
                usage_pct: usage_pct(new_chars, limit),
                usage_chars: new_chars,
                limit_chars: limit,
            },
            Err(e) => MemoryOpResult {
                ok: false,
                message: format!("Failed to write memory: {}", e),
                target: self.target_label(target),
                usage_pct: usage_pct(current_chars, limit),
                usage_chars: current_chars,
                limit_chars: limit,
            },
        }
    }

    pub fn remove(&self, target: MemoryTarget, old_text: &str) -> MemoryOpResult {
        let path = self.path_for(target);
        let limit = self.limit_for(target);
        let old_text = old_text.trim();

        if old_text.is_empty() {
            return MemoryOpResult {
                ok: false,
                message: "old_text must not be empty.".to_string(),
                target: self.target_label(target),
                usage_pct: 0,
                usage_chars: 0,
                limit_chars: limit,
            };
        }

        let existing = Self::read_file_content(path).unwrap_or_default();
        let entries = split_entries(&existing);
        let current_chars = existing.chars().count();

        let matches: Vec<(usize, &str)> = entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.contains(old_text))
            .map(|(i, e)| (i, *e))
            .collect();

        if matches.is_empty() {
            return MemoryOpResult {
                ok: false,
                message: format!(
                    "No entry in {} contains \"{}\".",
                    self.target_name(target),
                    old_text
                ),
                target: self.target_label(target),
                usage_pct: usage_pct(current_chars, limit),
                usage_chars: current_chars,
                limit_chars: limit,
            };
        }

        if matches.len() > 1 {
            let matched_snippets: Vec<String> = matches
                .iter()
                .map(|(_, e)| format!("  - \"{}\"", e))
                .collect();
            return MemoryOpResult {
                ok: false,
                message: format!(
                    "\"{}\" matches {} entries in {}. Provide a more specific old_text:\n{}",
                    old_text,
                    matches.len(),
                    self.target_name(target),
                    matched_snippets.join("\n")
                ),
                target: self.target_label(target),
                usage_pct: usage_pct(current_chars, limit),
                usage_chars: current_chars,
                limit_chars: limit,
            };
        }

        let (idx, _old_entry) = matches[0];
        let mut new_entries: Vec<&str> = entries.iter().copied().collect();
        new_entries.remove(idx);

        let new_file_content = new_entries.join(ENTRY_DELIMITER);
        let new_chars = new_file_content.chars().count();

        match Self::atomic_write(path, &new_file_content) {
            Ok(()) => MemoryOpResult {
                ok: true,
                message: format!(
                    "Removed entry from {} matching \"{}\".",
                    self.target_name(target),
                    old_text
                ),
                target: self.target_label(target),
                usage_pct: usage_pct(new_chars, limit),
                usage_chars: new_chars,
                limit_chars: limit,
            },
            Err(e) => MemoryOpResult {
                ok: false,
                message: format!("Failed to write memory: {}", e),
                target: self.target_label(target),
                usage_pct: usage_pct(current_chars, limit),
                usage_chars: current_chars,
                limit_chars: limit,
            },
        }
    }

    pub fn memory_usage(&self, target: MemoryTarget) -> MemoryUsage {
        let path = self.path_for(target);
        let limit = self.limit_for(target);
        let chars = Self::read_file_content(path)
            .map(|c| c.chars().count())
            .unwrap_or(0);
        MemoryUsage {
            pct: usage_pct(chars, limit),
            chars,
            limit_chars: limit,
        }
    }

    pub fn list_entries(&self, target: MemoryTarget) -> Vec<String> {
        let path = self.path_for(target);
        let content = Self::read_file_content(path).unwrap_or_default();
        split_entries(&content)
            .into_iter()
            .map(String::from)
            .collect()
    }

    // ── private helpers ──

    fn path_for(&self, target: MemoryTarget) -> &Path {
        match target {
            MemoryTarget::Memory => &self.memory_path,
            MemoryTarget::User => &self.user_path,
        }
    }

    fn limit_for(&self, target: MemoryTarget) -> usize {
        match target {
            MemoryTarget::Memory => MEMORY_MD_CHAR_LIMIT,
            MemoryTarget::User => USER_MD_CHAR_LIMIT,
        }
    }

    fn target_name(&self, target: MemoryTarget) -> &str {
        match target {
            MemoryTarget::Memory => "memory",
            MemoryTarget::User => "user",
        }
    }

    fn target_label(&self, target: MemoryTarget) -> MemoryTarget {
        target
    }

    fn read_file_content(path: &Path) -> Option<String> {
        match fs::read_to_string(path) {
            Ok(content) => {
                let trimmed = content.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(content)
                }
            }
            Err(_) => None,
        }
    }

    fn atomic_write(path: &Path, content: &str) -> io::Result<()> {
        let dir = path.parent().unwrap();
        fs::create_dir_all(dir)?;

        let temp_name = format!(".tmp_{}", uuid::Uuid::new_v4().simple());
        let temp_path = dir.join(&temp_name);

        let mut file = fs::File::create(&temp_path)?;
        file.write_all(content.as_bytes())?;
        file.sync_all()?;

        fs::rename(&temp_path, path)?;
        Ok(())
    }
}

// ── free functions ──

fn split_entries(content: &str) -> Vec<&str> {
    content
        .split(ENTRY_DELIMITER)
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect()
}

fn usage_pct(chars: usize, limit: usize) -> u32 {
    if limit == 0 {
        return 0;
    }
    let pct = (chars as f64 / limit as f64) * 100.0;
    (pct.round() as u32).min(100)
}

fn section_header(line: &str) -> String {
    let total_width: usize = 80;
    let line_len = line.chars().count();
    if line_len >= total_width {
        return line.to_string();
    }
    let remaining = total_width - line_len;
    let left = remaining / 2;
    let right = remaining - left;
    format!("{}{}{}", "═".repeat(left), line, "═".repeat(right))
}
