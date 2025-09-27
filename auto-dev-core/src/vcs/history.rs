//! Efficient commit history search and analysis

use anyhow::{Context, Result};
use git2::{Commit, Diff, DiffOptions, Oid, Repository, Time};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use crate::info;

/// Commit history search and analysis
pub struct HistorySearcher {
    repo: Repository,
    repo_path: PathBuf,
}

impl HistorySearcher {
    /// Create new history searcher
    pub fn new(repo_path: impl AsRef<Path>) -> Result<Self> {
        let repo_path = repo_path.as_ref().to_path_buf();
        let repo = Repository::open(&repo_path)
            .context("Failed to open git repository")?;
        
        Ok(Self { repo, repo_path })
    }

    /// Search commits by message pattern
    pub fn search_by_message(&self, pattern: &str, limit: Option<usize>) -> Result<Vec<CommitInfo>> {
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;
        
        let mut results = Vec::new();
        let pattern_lower = pattern.to_lowercase();
        
        for oid in revwalk {
            let oid = oid?;
            let commit = self.repo.find_commit(oid)?;
            
            if let Some(message) = commit.message() {
                if message.to_lowercase().contains(&pattern_lower) {
                    results.push(CommitInfo::from_commit(&commit)?);
                    
                    if let Some(limit) = limit {
                        if results.len() >= limit {
                            break;
                        }
                    }
                }
            }
        }
        
        Ok(results)
    }

    /// Search commits by author
    pub fn search_by_author(&self, author: &str, limit: Option<usize>) -> Result<Vec<CommitInfo>> {
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;
        
        let mut results = Vec::new();
        let author_lower = author.to_lowercase();
        
        for oid in revwalk {
            let oid = oid?;
            let commit = self.repo.find_commit(oid)?;
            
            let commit_author = commit.author().name().unwrap_or("").to_lowercase();
            if commit_author.contains(&author_lower) {
                results.push(CommitInfo::from_commit(&commit)?);
                
                if let Some(limit) = limit {
                    if results.len() >= limit {
                        break;
                    }
                }
            }
        }
        
        Ok(results)
    }

    /// Search commits that modified a file
    pub fn search_by_file(&self, file_path: &str, limit: Option<usize>) -> Result<Vec<CommitInfo>> {
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;
        
        let mut results = Vec::new();
        let mut last_oid: Option<Oid> = None;
        
        for oid in revwalk {
            let oid = oid?;
            let commit = self.repo.find_commit(oid)?;
            
            if self.commit_touches_file(&commit, file_path, last_oid)? {
                results.push(CommitInfo::from_commit(&commit)?);
                
                if let Some(limit) = limit {
                    if results.len() >= limit {
                        break;
                    }
                }
            }
            
            last_oid = Some(oid);
        }
        
        Ok(results)
    }

    /// Search commits within a date range
    pub fn search_by_date_range(
        &self,
        after: Option<i64>,
        before: Option<i64>,
    ) -> Result<Vec<CommitInfo>> {
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;
        
        let mut results = Vec::new();
        
        for oid in revwalk {
            let oid = oid?;
            let commit = self.repo.find_commit(oid)?;
            let commit_time = commit.time().seconds();
            
            let in_range = match (after, before) {
                (Some(a), Some(b)) => commit_time >= a && commit_time <= b,
                (Some(a), None) => commit_time >= a,
                (None, Some(b)) => commit_time <= b,
                (None, None) => true,
            };
            
            if in_range {
                results.push(CommitInfo::from_commit(&commit)?);
            }
        }
        
        Ok(results)
    }

    /// Find commits related to a keyword (in message, diff, or files)
    pub fn search_by_keyword(&self, keyword: &str, limit: Option<usize>) -> Result<Vec<CommitInfo>> {
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;
        
        let mut results = Vec::new();
        let keyword_lower = keyword.to_lowercase();
        
        for oid in revwalk {
            let oid = oid?;
            let commit = self.repo.find_commit(oid)?;
            
            // Check message
            if let Some(message) = commit.message() {
                if message.to_lowercase().contains(&keyword_lower) {
                    results.push(CommitInfo::from_commit(&commit)?);
                    
                    if let Some(limit) = limit {
                        if results.len() >= limit {
                            break;
                        }
                    }
                    continue;
                }
            }
            
            // Check diff content
            if self.commit_diff_contains(&commit, &keyword_lower)? {
                results.push(CommitInfo::from_commit(&commit)?);
                
                if let Some(limit) = limit {
                    if results.len() >= limit {
                        break;
                    }
                }
            }
        }
        
        Ok(results)
    }

    /// Get commit statistics for a time period
    pub fn get_commit_stats(&self, days: usize) -> Result<CommitStats> {
        let cutoff = chrono::Utc::now().timestamp() - (days as i64 * 86400);
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;
        
        let mut stats = CommitStats::default();
        let mut authors = HashMap::new();
        let mut daily_counts = HashMap::new();
        
        for oid in revwalk {
            let oid = oid?;
            let commit = self.repo.find_commit(oid)?;
            let commit_time = commit.time().seconds();
            
            if commit_time < cutoff {
                break;
            }
            
            stats.total_commits += 1;
            
            // Count by author
            let author = commit.author().name().unwrap_or("unknown").to_string();
            *authors.entry(author).or_insert(0) += 1;
            
            // Count by day
            let day = commit_time / 86400;
            *daily_counts.entry(day).or_insert(0) += 1;
        }
        
        stats.authors = authors;
        stats.average_per_day = if days > 0 {
            stats.total_commits as f64 / days as f64
        } else {
            0.0
        };
        
        Ok(stats)
    }

    /// Find commits that introduced or removed a pattern
    pub fn search_pattern_changes(&self, pattern: &str) -> Result<Vec<PatternChange>> {
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;
        
        let mut changes = Vec::new();
        
        for oid in revwalk {
            let oid = oid?;
            let commit = self.repo.find_commit(oid)?;
            
            if let Some(parent) = commit.parent(0).ok() {
                let diff = self.repo.diff_tree_to_tree(
                    Some(&parent.tree()?),
                    Some(&commit.tree()?),
                    None,
                )?;
                
                let pattern_change = self.analyze_pattern_in_diff(&diff, pattern)?;
                
                if pattern_change.added > 0 || pattern_change.removed > 0 {
                    changes.push(PatternChange {
                        commit_id: oid.to_string(),
                        message: commit.message().unwrap_or("").to_string(),
                        added: pattern_change.added,
                        removed: pattern_change.removed,
                        timestamp: commit.time().seconds(),
                    });
                }
            }
        }
        
        Ok(changes)
    }

    /// Get blame information for a file
    pub fn get_blame(&self, file_path: &str) -> Result<Vec<BlameLine>> {
        let blame = self.repo.blame_file(Path::new(file_path), None)?;
        let mut lines = Vec::new();
        
        for i in 0..blame.len() {
            let hunk = blame.get_index(i).unwrap();
            let commit = self.repo.find_commit(hunk.final_commit_id())?;
            
            for line_no in hunk.final_start_line()..hunk.final_start_line() + hunk.lines_in_hunk() {
                lines.push(BlameLine {
                    line_number: line_no,
                    commit_id: hunk.final_commit_id().to_string(),
                    author: commit.author().name().unwrap_or("unknown").to_string(),
                    timestamp: commit.time().seconds(),
                    message: commit.summary().unwrap_or("").to_string(),
                });
            }
        }
        
        Ok(lines)
    }

    // Helper methods

    fn commit_touches_file(&self, commit: &Commit, file_path: &str, last_oid: Option<Oid>) -> Result<bool> {
        let tree = commit.tree()?;
        
        let parent_tree = if let Some(parent_oid) = last_oid {
            let parent = self.repo.find_commit(parent_oid)?;
            Some(parent.tree()?)
        } else if commit.parent_count() > 0 {
            Some(commit.parent(0)?.tree()?)
        } else {
            None
        };
        
        let diff = self.repo.diff_tree_to_tree(
            parent_tree.as_ref(),
            Some(&tree),
            None,
        )?;
        
        let mut touched = false;
        diff.foreach(
            &mut |delta, _| {
                if let Some(path) = delta.new_file().path() {
                    if path == Path::new(file_path) {
                        touched = true;
                    }
                }
                true
            },
            None,
            None,
            None,
        )?;
        
        Ok(touched)
    }

    fn commit_diff_contains(&self, commit: &Commit, keyword: &str) -> Result<bool> {
        if commit.parent_count() == 0 {
            return Ok(false);
        }
        
        let parent = commit.parent(0)?;
        let diff = self.repo.diff_tree_to_tree(
            Some(&parent.tree()?),
            Some(&commit.tree()?),
            None,
        )?;
        
        let mut contains = false;
        diff.print(git2::DiffFormat::Patch, |_, _, line| {
            if let Ok(content) = std::str::from_utf8(line.content()) {
                if content.to_lowercase().contains(keyword) {
                    contains = true;
                }
            }
            true
        })?;
        
        Ok(contains)
    }

    fn analyze_pattern_in_diff(&self, diff: &Diff, pattern: &str) -> Result<PatternDiff> {
        let mut added = 0;
        let mut removed = 0;
        
        diff.print(git2::DiffFormat::Patch, |_, _, line| {
            if let Ok(content) = std::str::from_utf8(line.content()) {
                let count = content.matches(pattern).count();
                match line.origin() {
                    '+' => added += count,
                    '-' => removed += count,
                    _ => {}
                }
            }
            true
        })?;
        
        Ok(PatternDiff { added, removed })
    }
}

/// Commit information
#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub id: String,
    pub author: String,
    pub email: String,
    pub message: String,
    pub summary: String,
    pub timestamp: i64,
    pub parent_ids: Vec<String>,
}

impl CommitInfo {
    fn from_commit(commit: &Commit) -> Result<Self> {
        Ok(Self {
            id: commit.id().to_string(),
            author: commit.author().name().unwrap_or("unknown").to_string(),
            email: commit.author().email().unwrap_or("unknown").to_string(),
            message: commit.message().unwrap_or("").to_string(),
            summary: commit.summary().unwrap_or("").to_string(),
            timestamp: commit.time().seconds(),
            parent_ids: (0..commit.parent_count())
                .map(|i| commit.parent_id(i).map(|id| id.to_string()))
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
}

/// Commit statistics
#[derive(Debug, Default)]
pub struct CommitStats {
    pub total_commits: usize,
    pub authors: HashMap<String, usize>,
    pub average_per_day: f64,
}

/// Pattern change in a commit
#[derive(Debug)]
pub struct PatternChange {
    pub commit_id: String,
    pub message: String,
    pub added: usize,
    pub removed: usize,
    pub timestamp: i64,
}

/// Pattern diff statistics
struct PatternDiff {
    added: usize,
    removed: usize,
}

/// Blame information for a line
#[derive(Debug)]
pub struct BlameLine {
    pub line_number: usize,
    pub commit_id: String,
    pub author: String,
    pub timestamp: i64,
    pub message: String,
}

/// Advanced search builder
pub struct SearchBuilder<'a> {
    searcher: &'a HistorySearcher,
    message_pattern: Option<String>,
    author_pattern: Option<String>,
    file_pattern: Option<String>,
    after_date: Option<i64>,
    before_date: Option<i64>,
    limit: Option<usize>,
}

impl<'a> SearchBuilder<'a> {
    pub fn new(searcher: &'a HistorySearcher) -> Self {
        Self {
            searcher,
            message_pattern: None,
            author_pattern: None,
            file_pattern: None,
            after_date: None,
            before_date: None,
            limit: None,
        }
    }

    pub fn with_message(mut self, pattern: &str) -> Self {
        self.message_pattern = Some(pattern.to_string());
        self
    }

    pub fn with_author(mut self, pattern: &str) -> Self {
        self.author_pattern = Some(pattern.to_string());
        self
    }

    pub fn with_file(mut self, path: &str) -> Self {
        self.file_pattern = Some(path.to_string());
        self
    }

    pub fn after(mut self, timestamp: i64) -> Self {
        self.after_date = Some(timestamp);
        self
    }

    pub fn before(mut self, timestamp: i64) -> Self {
        self.before_date = Some(timestamp);
        self
    }

    pub fn limit(mut self, count: usize) -> Self {
        self.limit = Some(count);
        self
    }

    pub fn search(&self) -> Result<Vec<CommitInfo>> {
        let mut results = Vec::new();
        let mut revwalk = self.searcher.repo.revwalk()?;
        revwalk.push_head()?;
        
        for oid in revwalk {
            let oid = oid?;
            let commit = self.searcher.repo.find_commit(oid)?;
            
            // Check all filters
            if !self.matches_filters(&commit)? {
                continue;
            }
            
            results.push(CommitInfo::from_commit(&commit)?);
            
            if let Some(limit) = self.limit {
                if results.len() >= limit {
                    break;
                }
            }
        }
        
        Ok(results)
    }

    fn matches_filters(&self, commit: &Commit) -> Result<bool> {
        // Check message
        if let Some(ref pattern) = self.message_pattern {
            if let Some(message) = commit.message() {
                if !message.to_lowercase().contains(&pattern.to_lowercase()) {
                    return Ok(false);
                }
            } else {
                return Ok(false);
            }
        }
        
        // Check author
        if let Some(ref pattern) = self.author_pattern {
            let author = commit.author().name().unwrap_or("").to_lowercase();
            if !author.contains(&pattern.to_lowercase()) {
                return Ok(false);
            }
        }
        
        // Check date range
        let commit_time = commit.time().seconds();
        if let Some(after) = self.after_date {
            if commit_time < after {
                return Ok(false);
            }
        }
        if let Some(before) = self.before_date {
            if commit_time > before {
                return Ok(false);
            }
        }
        
        // File check would be expensive, skip for now in filter
        // Could be added with caching
        
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_builder() {
        // This is a compile test to ensure the builder pattern works
        let searcher = HistorySearcher {
            repo: unsafe { std::mem::zeroed() },
            repo_path: PathBuf::new(),
        };
        
        let _builder = SearchBuilder::new(&searcher)
            .with_message("fix")
            .with_author("alice")
            .limit(10);
    }
}
