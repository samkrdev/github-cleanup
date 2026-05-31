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

#[cfg(test)]
mod tests {
    use super::*;

    fn repo(name: &str, description: Option<&str>) -> Repo {
        Repo {
            name: name.to_string(),
            full_name: format!("octocat/{}", name),
            private: false,
            fork: false,
            archived: false,
            description: description.map(|d| d.to_string()),
            stargazers_count: 0,
            updated_at: "2024-01-01T00:00:00Z".to_string(),
            html_url: format!("https://github.com/octocat/{}", name),
        }
    }

    fn app_with(names: &[&str]) -> App {
        App::new(names.iter().map(|n| repo(n, None)).collect())
    }

    #[test]
    fn new_initializes_with_all_repos_visible() {
        let app = app_with(&["a", "b", "c"]);
        assert_eq!(app.filtered, vec![0, 1, 2]);
        assert_eq!(app.cursor, 0);
        assert!(app.selected.is_empty());
        assert_eq!(app.mode, Mode::Browsing);
        assert!(!app.should_quit);
    }

    #[test]
    fn current_repo_tracks_cursor_through_filter() {
        let mut app = app_with(&["a", "b", "c"]);
        assert_eq!(app.current_repo().unwrap().name, "a");
        app.cursor = 2;
        assert_eq!(app.current_repo().unwrap().name, "c");
    }

    #[test]
    fn current_repo_is_none_when_empty() {
        let app = app_with(&[]);
        assert!(app.current_repo().is_none());
    }

    #[test]
    fn move_cursor_wraps_around_both_ends() {
        let mut app = app_with(&["a", "b", "c"]);
        app.move_cursor(1);
        assert_eq!(app.cursor, 1);
        app.move_cursor(-2); // 1 -> -1 -> wraps to 2
        assert_eq!(app.cursor, 2);
        app.move_cursor(1); // 2 -> wraps to 0
        assert_eq!(app.cursor, 0);
    }

    #[test]
    fn move_cursor_with_large_delta_wraps_via_modulo() {
        let mut app = app_with(&["a", "b", "c"]);
        app.move_cursor(10); // 10 % 3 == 1
        assert_eq!(app.cursor, 1);
    }

    #[test]
    fn move_cursor_resets_to_zero_when_empty() {
        let mut app = app_with(&[]);
        app.cursor = 5;
        app.move_cursor(1);
        assert_eq!(app.cursor, 0);
    }

    #[test]
    fn jump_top_and_bottom() {
        let mut app = app_with(&["a", "b", "c"]);
        app.jump_bottom();
        assert_eq!(app.cursor, 2);
        app.jump_top();
        assert_eq!(app.cursor, 0);
    }

    #[test]
    fn jump_bottom_is_noop_when_empty() {
        let mut app = app_with(&[]);
        app.jump_bottom();
        assert_eq!(app.cursor, 0);
    }

    #[test]
    fn toggle_selected_adds_then_removes() {
        let mut app = app_with(&["a", "b"]);
        app.toggle_selected();
        assert!(app.selected.contains("octocat/a"));
        app.toggle_selected();
        assert!(!app.selected.contains("octocat/a"));
    }

    #[test]
    fn toggle_selected_is_noop_when_empty() {
        let mut app = app_with(&[]);
        app.toggle_selected();
        assert!(app.selected.is_empty());
    }

    #[test]
    fn clear_selection_empties_set() {
        let mut app = app_with(&["a", "b"]);
        app.toggle_selected();
        app.cursor = 1;
        app.toggle_selected();
        assert_eq!(app.selected.len(), 2);
        app.clear_selection();
        assert!(app.selected.is_empty());
    }

    #[test]
    fn apply_filter_matches_by_name() {
        let mut app = app_with(&["alpha", "beta", "gamma"]);
        app.filter = "et".to_string();
        app.apply_filter();
        assert_eq!(app.filtered, vec![1]);
    }

    #[test]
    fn apply_filter_is_case_insensitive() {
        let mut app = app_with(&["Alpha", "Beta"]);
        app.filter = "ALPHA".to_string();
        app.apply_filter();
        assert_eq!(app.filtered, vec![0]);
    }

    #[test]
    fn apply_filter_matches_full_name() {
        let mut app = app_with(&["alpha"]);
        app.filter = "octocat".to_string();
        app.apply_filter();
        assert_eq!(app.filtered, vec![0]);
    }

    #[test]
    fn apply_filter_matches_description() {
        let mut app = App::new(vec![
            repo("a", Some("a command line tool")),
            repo("b", None),
        ]);
        app.filter = "command".to_string();
        app.apply_filter();
        assert_eq!(app.filtered, vec![0]);
    }

    #[test]
    fn apply_filter_empty_query_shows_all() {
        let mut app = app_with(&["a", "b", "c"]);
        app.filter = "a".to_string();
        app.apply_filter();
        app.filter.clear();
        app.apply_filter();
        assert_eq!(app.filtered, vec![0, 1, 2]);
    }

    #[test]
    fn apply_filter_clamps_cursor_into_range() {
        let mut app = app_with(&["alpha", "beta", "gamma"]);
        app.cursor = 2;
        app.filter = "alpha".to_string();
        app.apply_filter();
        assert_eq!(app.filtered, vec![0]);
        assert_eq!(app.cursor, 0);
    }

    #[test]
    fn apply_filter_no_match_leaves_empty() {
        let mut app = app_with(&["alpha", "beta"]);
        app.filter = "zzz".to_string();
        app.apply_filter();
        assert!(app.filtered.is_empty());
        assert_eq!(app.cursor, 0);
    }

    #[test]
    fn selected_repos_returns_selected_full_names() {
        let mut app = app_with(&["a", "b"]);
        app.toggle_selected();
        let names = app.selected_repos();
        assert_eq!(names, vec!["octocat/a".to_string()]);
    }

    #[test]
    fn remove_repos_drops_repos_and_selection() {
        let mut app = app_with(&["a", "b", "c"]);
        app.toggle_selected(); // selects octocat/a
        let mut to_remove = HashSet::new();
        to_remove.insert("octocat/a".to_string());
        app.remove_repos(&to_remove);

        assert_eq!(app.repos.len(), 2);
        assert!(app.repos.iter().all(|r| r.full_name != "octocat/a"));
        assert!(!app.selected.contains("octocat/a"));
        assert_eq!(app.filtered, vec![0, 1]);
    }

    #[test]
    fn remove_repos_reapplies_active_filter() {
        let mut app = app_with(&["alpha", "alphabet", "beta"]);
        app.filter = "alpha".to_string();
        app.apply_filter();
        assert_eq!(app.filtered, vec![0, 1]);

        let mut to_remove = HashSet::new();
        to_remove.insert("octocat/alpha".to_string());
        app.remove_repos(&to_remove);

        // "alphabet" remains and still matches the filter; "beta" filtered out.
        assert_eq!(app.repos.len(), 2);
        assert_eq!(app.filtered.len(), 1);
        assert_eq!(app.repos[app.filtered[0]].name, "alphabet");
    }
}
