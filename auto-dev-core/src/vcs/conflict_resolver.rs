//! Conflict resolution for git merges

use anyhow::{Context, Result};
use git2::{Repository, Status, StatusOptions};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

use super::ConflictResolution;

/// Handles git merge conflicts
pub struct ConflictResolver {
    repo: Repository,
    repo_path: PathBuf,
}

impl ConflictResolver {
    /// Create new conflict resolver
    pub fn new(repo_path: impl AsRef<Path>) -> Result<Self> {
        let repo_path = repo_path.as_ref().to_path_buf();
        let repo = Repository::open(&repo_path)
            .context("Failed to open git repository")?;
        
        Ok(Self { repo, repo_path })
    }

    /// Attempt to resolve conflicts
    pub fn attempt_resolution(&self, max_attempts: usize) -> Result<ConflictResolution> {
        let conflicts = self.find_conflicts()?;
        
        if conflicts.is_empty() {
            return Ok(ConflictResolution::Resolved);
        }
        
        info!("Found {} conflicted files", conflicts.len());
        
        let mut unresolved = Vec::new();
        let mut attempts = 0;
        
        for conflict_path in conflicts {
            if attempts >= max_attempts {
                unresolved.push(conflict_path);
                continue;
            }
            
            match self.resolve_file(&conflict_path) {
                Ok(true) => {
                    info!("Resolved conflict in {:?}", conflict_path);
                }
                Ok(false) => {
                    warn!("Could not auto-resolve {:?}", conflict_path);
                    unresolved.push(conflict_path);
                }
                Err(e) => {
                    warn!("Error resolving {:?}: {}", conflict_path, e);
                    unresolved.push(conflict_path);
                }
            }
            
            attempts += 1;
        }
        
        if unresolved.is_empty() {
            Ok(ConflictResolution::Resolved)
        } else {
            Ok(ConflictResolution::RequiresManual(unresolved))
        }
    }

    /// Find all conflicted files
    pub fn find_conflicts(&self) -> Result<Vec<PathBuf>> {
        let mut opts = StatusOptions::new();
        opts.include_untracked(false);
        
        let statuses = self.repo.statuses(Some(&mut opts))?;
        let mut conflicts = Vec::new();
        
        for entry in statuses.iter() {
            if entry.status().contains(Status::CONFLICTED) {
                if let Some(path) = entry.path() {
                    conflicts.push(PathBuf::from(path));
                }
            }
        }
        
        Ok(conflicts)
    }

    /// Attempt to resolve a single file
    fn resolve_file(&self, path: &Path) -> Result<bool> {
        let full_path = self.repo_path.join(path);
        
        if !full_path.exists() {
            return Ok(false);
        }
        
        let content = fs::read_to_string(&full_path)?;
        
        // Check if it's a simple conflict we can resolve
        if let Some(resolved) = self.try_simple_resolution(&content) {
            fs::write(&full_path, resolved)?;
            self.mark_resolved(path)?;
            return Ok(true);
        }
        
        // Try strategy-based resolution
        if let Some(resolved) = self.try_strategy_resolution(&content, path) {
            fs::write(&full_path, resolved)?;
            self.mark_resolved(path)?;
            return Ok(true);
        }
        
        Ok(false)
    }

    /// Try simple conflict resolution
    fn try_simple_resolution(&self, content: &str) -> Option<String> {
        // Handle conflicts where one side is empty (additions)
        if content.contains("<<<<<<<") && content.contains("=======") && content.contains(">>>>>>>") {
            let lines: Vec<&str> = content.lines().collect();
            let mut result = Vec::new();
            let mut in_conflict = false;
            let mut ours: Vec<&str> = Vec::with_capacity(lines.len() / 2);
            let mut theirs: Vec<&str> = Vec::with_capacity(lines.len() / 2);
            let mut current_section = 0; // 0 = before, 1 = ours, 2 = theirs
            
            for line in lines {
                if line.starts_with("<<<<<<<") {
                    in_conflict = true;
                    current_section = 1;
                    ours = Vec::new();
                    theirs = Vec::new();
                } else if line == "=======" && in_conflict {
                    current_section = 2;
                } else if line.starts_with(">>>>>>>") && in_conflict {
                    // Decide resolution strategy
                    let ours_slice: Vec<String> = ours.cloned();
                    let theirs_slice: Vec<String> = theirs.cloned();
                    if let Some(resolved) = self.choose_resolution(&ours_slice, &theirs_slice) {
                        result.extend(resolved);
                    } else {
                        // Can't auto-resolve, keep conflict markers
                        return None;
                    }
                    in_conflict = false;
                    current_section = 0;
                } else if in_conflict {
                    match current_section {
                        1 => ours.push(line),
                        2 => theirs.push(line),
                        _ => {}
                    }
                } else {
                    result.push(line);
                }
            }
            
            if !in_conflict {
                return Some(result.join("\n"));
            }
        }
        
        None
    }

    /// Choose between conflicting versions
    fn choose_resolution<'a>(&self, ours: &'a [&str], theirs: &'a [&str]) -> Option<Vec<&'a str>> {
        // If one side is empty, take the other
        if ours.is_empty() && !theirs.is_empty() {
            return Some(theirs.to_vec());
        }
        if theirs.is_empty() && !ours.is_empty() {
            return Some(ours.to_vec());
        }
        
        // If both sides are identical, take one
        if ours == theirs {
            return Some(ours.to_vec());
        }
        
        // Check for simple addition conflicts (both added different things)
        // In this case, we can sometimes combine them
        if self.looks_like_additions(ours, theirs) {
            let mut combined = ours.to_vec();
            combined.extend(theirs);
            return Some(combined);
        }
        
        None
    }

    /// Check if conflicts look like independent additions
    fn looks_like_additions(&self, ours: &[&str], theirs: &[&str]) -> bool {
        // Simple heuristic: if they're different lengths and don't share lines,
        // they might be independent additions
        if ours.len() != theirs.len() {
            for our_line in ours {
                if theirs.contains(our_line) {
                    return false;
                }
            }
            return true;
        }
        false
    }

    /// Try strategy-based resolution based on file type
    fn try_strategy_resolution(&self, content: &str, path: &Path) -> Option<String> {
        let extension = path.extension()?.to_str()?;
        
        match extension {
            "toml" | "json" | "yaml" => {
                // For config files, might try to merge keys
                // For now, return None (manual resolution needed)
                None
            }
            "md" | "txt" => {
                // For documentation, might concatenate both versions
                // For now, return None
                None
            }
            _ => None,
        }
    }

    /// Mark a file as resolved
    fn mark_resolved(&self, path: &Path) -> Result<()> {
        let mut index = self.repo.index()?;
        index.add_path(path)?;
        index.write()?;
        Ok(())
    }

    /// Get conflict details for manual resolution
    pub fn get_conflict_details(&self, path: &Path) -> Result<ConflictDetails> {
        let full_path = self.repo_path.join(path);
        let content = fs::read_to_string(&full_path)?;
        
        let mut conflicts = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;
        
        while i < lines.len() {
            if lines[i].starts_with("<<<<<<<") {
                let start_line = i + 1;
                let mut ours_end = i;
                let mut theirs_start = i;
                let mut end_line = i;
                
                for j in i+1..lines.len() {
                    if lines[j] == "=======" {
                        ours_end = j;
                        theirs_start = j + 1;
                    } else if lines[j].starts_with(">>>>>>>") {
                        end_line = j;
                        break;
                    }
                }
                
                if ours_end > i && theirs_start > ours_end && end_line > theirs_start {
                    let ours: Vec<String> = lines[start_line..ours_end]
                        .iter()
                        .map(|s| s.to_string())
                        .collect();
                    let theirs: Vec<String> = lines[theirs_start..end_line]
                        .iter()
                        .map(|s| s.to_string())
                        .collect();
                    
                    conflicts.push(ConflictSection {
                        start_line,
                        end_line,
                        ours,
                        theirs,
                    });
                    
                    i = end_line;
                }
            }
            i += 1;
        }
        
        Ok(ConflictDetails {
            path: path.to_path_buf(),
            conflicts,
            total_lines: lines.len(),
        })
    }

    /// Apply manual resolution
    pub fn apply_resolution(&self, path: &Path, resolution: &str) -> Result<()> {
        let full_path = self.repo_path.join(path);
        fs::write(&full_path, resolution)?;
        self.mark_resolved(path)?;
        info!("Applied manual resolution to {:?}", path);
        Ok(())
    }
}

/// Details about conflicts in a file
#[derive(Debug, Clone)]
pub struct ConflictDetails {
    pub path: PathBuf,
    pub conflicts: Vec<ConflictSection>,
    pub total_lines: usize,
}

/// A single conflict section
#[derive(Debug, Clone)]
pub struct ConflictSection {
    pub start_line: usize,
    pub end_line: usize,
    pub ours: Vec<String>,
    pub theirs: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_resolution_empty_ours() {
        let resolver = ConflictResolver {
            repo: unsafe { std::mem::zeroed() }, // Mock for testing
            repo_path: PathBuf::new(),
        };
        
        let content = "line 1\n<<<<<<< HEAD\n=======\nnew line\n>>>>>>> branch\nline 2";
        let resolved = resolver.try_simple_resolution(content);
        
        assert!(resolved.is_some());
        let resolved = resolved.unwrap();
        assert!(resolved.contains("new line"));
        assert!(!resolved.contains("<<<<<<<"));
    }

    #[test]
    fn test_simple_resolution_identical() {
        let resolver = ConflictResolver {
            repo: unsafe { std::mem::zeroed() },
            repo_path: PathBuf::new(),
        };
        
        let content = "<<<<<<< HEAD\nsame line\n=======\nsame line\n>>>>>>> branch";
        let resolved = resolver.try_simple_resolution(content);
        
        assert!(resolved.is_some());
        let resolved = resolved.unwrap();
        assert!(resolved.contains("same line"));
        assert!(!resolved.contains("<<<<<<<"));
    }
}
