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
        let resp = self
            .client
            .delete(&url)
            .bearer_auth(&self.token)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()
            .await
            .context("delete request failed")?;
        let status = resp.status();
        if status == reqwest::StatusCode::NO_CONTENT {
            return Ok(());
        }
        let body = resp.text().await.unwrap_or_default();
        Err(anyhow!("delete failed ({}): {}", status, body))
    }
}
