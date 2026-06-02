use std::collections::BTreeMap;

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;

const USER_AGENT: &str = "github-cleanup-tui";

#[derive(Debug, Clone, Deserialize)]
pub struct Repo {
    pub name: String,
    pub full_name: String,
    pub private: bool,
    pub fork: bool,
    pub archived: bool,
    pub description: Option<String>,
    pub stargazers_count: u64,
    pub updated_at: String,
    pub html_url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Gist {
    pub id: String,
    pub description: Option<String>,
    pub public: bool,
    pub html_url: String,
    pub updated_at: String,
    #[serde(default)]
    pub files: BTreeMap<String, GistFile>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GistFile {
    pub filename: Option<String>,
}

impl Gist {
    /// A short, human-friendly label: the first filename, falling back to the id.
    pub fn display_name(&self) -> String {
        self.files
            .values()
            .filter_map(|f| f.filename.clone())
            .next()
            .or_else(|| self.files.keys().next().cloned())
            .unwrap_or_else(|| self.id.clone())
    }

    pub fn file_count(&self) -> usize {
        self.files.len()
    }
}

pub struct GithubClient {
    client: reqwest::Client,
    token: String,
}

impl GithubClient {
    pub fn new(token: String) -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .build()
            .context("failed to build http client")?;
        Ok(Self { client, token })
    }

    pub async fn list_repos(&self) -> Result<Vec<Repo>> {
        let mut all = Vec::new();
        let mut page = 1u32;
        loop {
            let url = format!(
                "https://api.github.com/user/repos?per_page=100&page={}&visibility=all&affiliation=owner&sort=updated",
                page
            );
            let resp = self
                .client
                .get(&url)
                .bearer_auth(&self.token)
                .header("Accept", "application/vnd.github+json")
                .header("X-GitHub-Api-Version", "2022-11-28")
                .send()
                .await
                .context("request to GitHub failed")?;

            let status = resp.status();
            if !status.is_success() {
                let body = resp.text().await.unwrap_or_default();
                return Err(anyhow!("GitHub API {}: {}", status, body));
            }
            let batch: Vec<Repo> = resp.json().await.context("invalid repo JSON")?;
            let done = batch.len() < 100;
            all.extend(batch);
            if done {
                break;
            }
            page += 1;
        }
        Ok(all)
    }

    pub async fn delete_repo(&self, full_name: &str) -> Result<()> {
        let url = format!("https://api.github.com/repos/{}", full_name);
        self.delete(&url, "delete").await
    }

    pub async fn list_gists(&self) -> Result<Vec<Gist>> {
        let mut all = Vec::new();
        let mut page = 1u32;
        loop {
            let url = format!("https://api.github.com/gists?per_page=100&page={}", page);
            let resp = self
                .client
                .get(&url)
                .bearer_auth(&self.token)
                .header("Accept", "application/vnd.github+json")
                .header("X-GitHub-Api-Version", "2022-11-28")
                .send()
                .await
                .context("request to GitHub failed")?;

            let status = resp.status();
            if !status.is_success() {
                let body = resp.text().await.unwrap_or_default();
                return Err(anyhow!("GitHub API {}: {}", status, body));
            }
            let batch: Vec<Gist> = resp.json().await.context("invalid gist JSON")?;
            let done = batch.len() < 100;
            all.extend(batch);
            if done {
                break;
            }
            page += 1;
        }
        Ok(all)
    }

    pub async fn delete_gist(&self, id: &str) -> Result<()> {
        let url = format!("https://api.github.com/gists/{}", id);
        self.delete(&url, "delete").await
    }

    async fn delete(&self, url: &str, what: &str) -> Result<()> {
        let resp = self
            .client
            .delete(url)
            .bearer_auth(&self.token)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()
            .await
            .with_context(|| format!("{} request failed", what))?;
        let status = resp.status();
        if status == reqwest::StatusCode::NO_CONTENT {
            return Ok(());
        }
        let body = resp.text().await.unwrap_or_default();
        Err(anyhow!("{} failed ({}): {}", what, status, body))
    }
}
