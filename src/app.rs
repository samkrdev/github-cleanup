use std::collections::HashSet;

use crate::github::Repo;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    Browsing,
    Filtering,
    Confirming,
    Deleting,
}

pub struct App {
    pub repos: Vec<Repo>,
    pub filtered: Vec<usize>,
    pub cursor: usize,
    pub selected: HashSet<String>,
    pub mode: Mode,
    pub filter: String,
    pub status: String,
    pub deletion_results: Vec<(String, Result<(), String>)>,
    pub should_quit: bool,
}

impl App {
    pub fn new(repos: Vec<Repo>) -> Self {
        let filtered = (0..repos.len()).collect();
        Self {
            repos,
            filtered,
            cursor: 0,
            selected: HashSet::new(),
            mode: Mode::Browsing,
            filter: String::new(),
            status: String::new(),
            deletion_results: Vec::new(),
            should_quit: false,
        }
    }

    pub fn current_repo(&self) -> Option<&Repo> {
        self.filtered
            .get(self.cursor)
            .and_then(|&i| self.repos.get(i))
    }

    pub fn move_cursor(&mut self, delta: isize) {
        if self.filtered.is_empty() {
            self.cursor = 0;
            return;
        }
        let len = self.filtered.len() as isize;
        let next = (self.cursor as isize + delta).rem_euclid(len);
        self.cursor = next as usize;
    }

    pub fn jump_top(&mut self) {
        self.cursor = 0;
    }

    pub fn jump_bottom(&mut self) {
        if !self.filtered.is_empty() {
            self.cursor = self.filtered.len() - 1;
        }
    }

    pub fn toggle_selected(&mut self) {
        if let Some(repo) = self.current_repo() {
            let key = repo.full_name.clone();
            if !self.selected.remove(&key) {
                self.selected.insert(key);
            }
        }
    }

    pub fn clear_selection(&mut self) {
        self.selected.clear();
    }

    pub fn apply_filter(&mut self) {
        let q = self.filter.to_lowercase();
        if q.is_empty() {
            self.filtered = (0..self.repos.len()).collect();
        } else {
            self.filtered = self
                .repos
                .iter()
                .enumerate()
                .filter(|(_, r)| {
                    r.name.to_lowercase().contains(&q)
                        || r.full_name.to_lowercase().contains(&q)
                        || r.description
                            .as_deref()
                            .map(|d| d.to_lowercase().contains(&q))
                            .unwrap_or(false)
                })
                .map(|(i, _)| i)
                .collect();
        }
        if self.cursor >= self.filtered.len() {
            self.cursor = self.filtered.len().saturating_sub(1);
        }
    }

    pub fn selected_repos(&self) -> Vec<String> {
        self.selected.iter().cloned().collect()
    }

    pub fn remove_repos(&mut self, names: &HashSet<String>) {
        self.repos.retain(|r| !names.contains(&r.full_name));
        self.selected.retain(|n| !names.contains(n));
        self.apply_filter();
    }
}
