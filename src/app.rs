use std::collections::HashSet;

use crate::github::{Gist, Repo};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    Browsing,
    Filtering,
    Confirming,
    Deleting,
}

/// Which collection the user is currently browsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Repos,
    Gists,
}

impl View {
    pub fn label(self) -> &'static str {
        match self {
            View::Repos => "Repositories",
            View::Gists => "Gists",
        }
    }
}

pub struct App {
    pub repos: Vec<Repo>,
    pub gists: Vec<Gist>,
    pub view: View,
    pub filtered: Vec<usize>,
    pub cursor: usize,
    pub selected: HashSet<String>,
    /// Selection stashed for the view that is not currently active.
    selected_other: HashSet<String>,
    pub mode: Mode,
    pub filter: String,
    pub status: String,
    /// One entry per attempted deletion: `(key, label, result)`.
    pub deletion_results: Vec<(String, String, Result<(), String>)>,
    pub should_quit: bool,
}

impl App {
    pub fn new(repos: Vec<Repo>) -> Self {
        let filtered = (0..repos.len()).collect();
        Self {
            repos,
            gists: Vec::new(),
            view: View::Repos,
            filtered,
            cursor: 0,
            selected: HashSet::new(),
            selected_other: HashSet::new(),
            mode: Mode::Browsing,
            filter: String::new(),
            status: String::new(),
            deletion_results: Vec::new(),
            should_quit: false,
        }
    }

    /// Number of items in the active view, before filtering.
    pub fn item_count(&self) -> usize {
        match self.view {
            View::Repos => self.repos.len(),
            View::Gists => self.gists.len(),
        }
    }

    /// Switch between the Repos and Gists views, preserving each view's
    /// selection and resetting the cursor/filter.
    pub fn toggle_view(&mut self) {
        self.view = match self.view {
            View::Repos => View::Gists,
            View::Gists => View::Repos,
        };
        std::mem::swap(&mut self.selected, &mut self.selected_other);
        self.filter.clear();
        self.cursor = 0;
        self.apply_filter();
    }

    pub fn current_repo(&self) -> Option<&Repo> {
        self.filtered
            .get(self.cursor)
            .and_then(|&i| self.repos.get(i))
    }

    pub fn current_gist(&self) -> Option<&Gist> {
        self.filtered
            .get(self.cursor)
            .and_then(|&i| self.gists.get(i))
    }

    /// Stable key for the item under the cursor: a repo's full name or a gist's id.
    fn current_key(&self) -> Option<String> {
        match self.view {
            View::Repos => self.current_repo().map(|r| r.full_name.clone()),
            View::Gists => self.current_gist().map(|g| g.id.clone()),
        }
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
        if let Some(key) = self.current_key() {
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
        self.filtered = match self.view {
            View::Repos => Self::filter_indices(self.repos.iter(), &q, |r| {
                r.name.to_lowercase().contains(&q)
                    || r.full_name.to_lowercase().contains(&q)
                    || r.description
                        .as_deref()
                        .map(|d| d.to_lowercase().contains(&q))
                        .unwrap_or(false)
            }),
            View::Gists => Self::filter_indices(self.gists.iter(), &q, |g| {
                g.display_name().to_lowercase().contains(&q)
                    || g.files
                        .keys()
                        .any(|name| name.to_lowercase().contains(&q))
                    || g.description
                        .as_deref()
                        .map(|d| d.to_lowercase().contains(&q))
                        .unwrap_or(false)
            }),
        };
        if self.cursor >= self.filtered.len() {
            self.cursor = self.filtered.len().saturating_sub(1);
        }
    }

    fn filter_indices<'a, T, I, F>(items: I, q: &str, matches: F) -> Vec<usize>
    where
        I: Iterator<Item = &'a T>,
        T: 'a,
        F: Fn(&T) -> bool,
    {
        items
            .enumerate()
            .filter(|(_, item)| q.is_empty() || matches(item))
            .map(|(i, _)| i)
            .collect()
    }

    /// Keys of the currently-selected items (repo full names or gist ids).
    pub fn selected_keys(&self) -> Vec<String> {
        self.selected.iter().cloned().collect()
    }

    /// Selected items as `(key, human-readable label)`, sorted by label.
    pub fn selected_labeled(&self) -> Vec<(String, String)> {
        let mut out: Vec<(String, String)> = self
            .selected
            .iter()
            .map(|key| (key.clone(), self.label_for(key)))
            .collect();
        out.sort_by(|a, b| a.1.cmp(&b.1));
        out
    }

    /// A display label for a selection key in the active view.
    pub fn label_for(&self, key: &str) -> String {
        match self.view {
            View::Repos => key.to_string(),
            View::Gists => self
                .gists
                .iter()
                .find(|g| g.id == key)
                .map(|g| g.display_name())
                .unwrap_or_else(|| key.to_string()),
        }
    }

    /// Drop the items whose key is in `keys` from the active view, and update
    /// selection and filtering accordingly.
    pub fn remove_selected(&mut self, keys: &HashSet<String>) {
        match self.view {
            View::Repos => self.repos.retain(|r| !keys.contains(&r.full_name)),
            View::Gists => self.gists.retain(|g| !keys.contains(&g.id)),
        }
        self.selected.retain(|k| !keys.contains(k));
        self.apply_filter();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::GistFile;
    use std::collections::BTreeMap;

    fn gist(id: &str, files: &[&str], description: Option<&str>) -> Gist {
        let mut map = BTreeMap::new();
        for f in files {
            map.insert(
                f.to_string(),
                GistFile {
                    filename: Some(f.to_string()),
                },
            );
        }
        Gist {
            id: id.to_string(),
            description: description.map(|d| d.to_string()),
            public: true,
            html_url: format!("https://gist.github.com/{}", id),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
            files: map,
        }
    }

    #[test]
    fn gist_display_name_uses_first_file() {
        let g = gist("abc", &["notes.md", "snippet.rs"], None);
        assert_eq!(g.display_name(), "notes.md");
        assert_eq!(g.file_count(), 2);
    }

    #[test]
    fn gist_display_name_falls_back_to_id() {
        let g = gist("abc", &[], None);
        assert_eq!(g.display_name(), "abc");
    }

    #[test]
    fn toggle_view_switches_collection_and_stashes_selection() {
        let mut app = app_with(&["a", "b"]);
        app.gists = vec![gist("g1", &["x.rs"], None)];
        app.toggle_selected(); // select octocat/a in Repos view
        assert!(app.selected.contains("octocat/a"));

        app.toggle_view();
        assert_eq!(app.view, View::Gists);
        assert_eq!(app.filtered, vec![0]); // the one gist
        assert!(app.selected.is_empty()); // repo selection stashed away

        app.toggle_selected(); // select g1
        assert!(app.selected.contains("g1"));

        app.toggle_view();
        assert_eq!(app.view, View::Repos);
        assert!(app.selected.contains("octocat/a")); // restored
        assert!(!app.selected.contains("g1"));
    }

    #[test]
    fn apply_filter_matches_gist_filename_and_description() {
        let mut app = app_with(&[]);
        app.gists = vec![
            gist("g1", &["deploy.sh"], Some("ci helper")),
            gist("g2", &["readme.md"], None),
        ];
        app.toggle_view();
        app.filter = "deploy".to_string();
        app.apply_filter();
        assert_eq!(app.filtered, vec![0]);

        app.filter = "ci".to_string();
        app.apply_filter();
        assert_eq!(app.filtered, vec![0]);
    }

    #[test]
    fn remove_selected_drops_gists_in_gist_view() {
        let mut app = app_with(&[]);
        app.gists = vec![gist("g1", &["a"], None), gist("g2", &["b"], None)];
        app.toggle_view();
        let mut to_remove = HashSet::new();
        to_remove.insert("g1".to_string());
        app.remove_selected(&to_remove);
        assert_eq!(app.gists.len(), 1);
        assert_eq!(app.gists[0].id, "g2");
    }

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
    fn selected_keys_returns_selected_full_names() {
        let mut app = app_with(&["a", "b"]);
        app.toggle_selected();
        let names = app.selected_keys();
        assert_eq!(names, vec!["octocat/a".to_string()]);
    }

    #[test]
    fn remove_selected_drops_repos_and_selection() {
        let mut app = app_with(&["a", "b", "c"]);
        app.toggle_selected(); // selects octocat/a
        let mut to_remove = HashSet::new();
        to_remove.insert("octocat/a".to_string());
        app.remove_selected(&to_remove);

        assert_eq!(app.repos.len(), 2);
        assert!(app.repos.iter().all(|r| r.full_name != "octocat/a"));
        assert!(!app.selected.contains("octocat/a"));
        assert_eq!(app.filtered, vec![0, 1]);
    }

    #[test]
    fn remove_selected_reapplies_active_filter() {
        let mut app = app_with(&["alpha", "alphabet", "beta"]);
        app.filter = "alpha".to_string();
        app.apply_filter();
        assert_eq!(app.filtered, vec![0, 1]);

        let mut to_remove = HashSet::new();
        to_remove.insert("octocat/alpha".to_string());
        app.remove_selected(&to_remove);

        // "alphabet" remains and still matches the filter; "beta" filtered out.
        assert_eq!(app.repos.len(), 2);
        assert_eq!(app.filtered.len(), 1);
        assert_eq!(app.repos[app.filtered[0]].name, "alphabet");
    }
}
