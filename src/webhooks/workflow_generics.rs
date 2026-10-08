use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub id: i64,
    pub node_id: String,
    pub name: String,
    pub full_name: String,
    pub private: bool,
    pub fork: bool,
    pub description: Option<String>,
    pub owner: User,
    pub archive_url: String,
    pub assignees_url: String,
    pub blobs_url: String,
    pub branches_url: String,
    pub collaborators_url: String,
    pub comments_url: String,
    pub commits_url: String,
    pub compare_url: String,
    pub contents_url: String,
    pub contributors_url: String,
    pub deployments_url: String,
    pub downloads_url: String,
    pub events_url: String,
    pub forks_url: String,
    pub git_commits_url: String,
    pub git_refs_url: String,
    pub git_tags_url: String,
    pub hooks_url: String,
    pub html_url: String,
    pub issue_comment_url: String,
    pub issue_events_url: String,
    pub issues_url: String,
    pub keys_url: String,
    pub labels_url: String,
    pub languages_url: String,
    pub merges_url: String,
    pub milestones_url: String,
    pub notifications_url: String,
    pub pulls_url: String,
    pub releases_url: String,
    pub stargazers_url: String,
    pub statuses_url: String,
    pub subscribers_url: String,
    pub subscription_url: String,
    pub tags_url: String,
    pub teams_url: String,
    pub trees_url: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub login: String,
    pub id: i64,
    pub node_id: String,
    #[serde(rename = "type")]
    pub user_type: UserType,
    pub site_admin: bool,
    pub gravatar_id: String,
    pub name: Option<String>,
    pub email: Option<String>,
    pub avatar_url: String,
    pub url: String,
    pub html_url: String,
    pub followers_url: String,
    pub following_url: String,
    pub gists_url: String,
    pub starred_url: String,
    pub subscriptions_url: String,
    pub organizations_url: String,
    pub repos_url: String,
    pub events_url: String,
    pub received_events_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UserType {
    Bot,
    User,
    Organization,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commit {
    pub id: String,
    pub tree_id: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub author: Committer,
    pub committer: Committer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Committer {
    pub name: String,
    pub email: Option<String>,
    pub date: Option<DateTime<Utc>>,
    pub username: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub login: String,
    pub id: i64,
    pub node_id: String,
    pub description: Option<String>,
    pub url: String,
    pub html_url: Option<String>,
    pub repos_url: String,
    pub events_url: String,
    pub hooks_url: String,
    pub issues_url: String,
    pub members_url: String,
    pub public_members_url: String,
    pub avatar_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Installation {
    pub id: i64,
    pub node_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequest {
    pub url: String,
    pub id: i64,
    pub number: i32,
    pub head: PullRequestRef,
    pub base: PullRequestRef,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequestRef {
    pub r#ref: String,
    pub sha: String,
    pub repo: RepoRef,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoRef {
    pub id: i64,
    pub url: String,
    pub name: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    const REAL_RUN: &str = include_str!("../../tests/fixtures/workflow_run.json");

    fn run() -> Value {
        serde_json::from_str(REAL_RUN).expect("fixture must be valid JSON")
    }

    fn parse<T: serde::de::DeserializeOwned>(label: &str, v: &Value) -> T {
        serde_json::from_value(v.clone())
            .unwrap_or_else(|e| panic!("{label} must deserialize, got: {e}"))
    }

    #[test]
    fn every_generic_object_in_the_real_payload_deserializes() {
        let r = run();
        let _: User = parse("actor", &r["actor"]);
        let _: User = parse("triggering_actor", &r["triggering_actor"]);
        let _: User = parse("repository.owner", &r["repository"]["owner"]);
        let _: Repository = parse("repository", &r["repository"]);
        let _: Repository = parse("head_repository", &r["head_repository"]);
        let _: Commit = parse("head_commit", &r["head_commit"]);
        let _: Committer = parse("head_commit.author", &r["head_commit"]["author"]);
        let _: PullRequest = parse("pull_requests[0]", &r["pull_requests"][0]);
    }

    #[test]
    fn user_name_is_absent_from_real_payloads() {
        let r = run();
        for pos in ["actor", "triggering_actor"] {
            assert!(
                !r[pos].as_object().unwrap().contains_key("name"),
                "{pos} unexpectedly has a `name`"
            );
            let user: User = parse(pos, &r[pos]);
            assert!(user.name.is_none(), "{pos}");
        }
    }

    #[test]
    fn commit_carries_both_author_and_committer() {
        let commit: Commit = parse("head_commit", &run()["head_commit"]);
        assert_eq!(commit.author.name, "ci-bot[bot]");
        assert_eq!(commit.committer.name, "GitHub");
        assert_eq!(commit.id.len(), 40, "`id` is a commit SHA, not a number");
        assert_eq!(commit.timestamp.to_rfc3339(), "2026-10-07T10:50:49+00:00");
    }

    #[test]
    fn committer_date_and_username_are_absent_on_the_verified_run() {
        let commit: Commit = parse("head_commit", &run()["head_commit"]);
        assert!(commit.author.date.is_none());
        assert!(commit.author.username.is_none());
        assert_eq!(commit.author.email.as_deref(), Some("ci@example.invalid"));
    }

    #[test]
    fn pull_request_head_and_base_carry_ref_sha_and_repo() {
        let pr: PullRequest = parse("pull_requests[0]", &run()["pull_requests"][0]);
        assert_eq!(pr.number, 60);
        assert_eq!(pr.head.r#ref, "renovate/amazon-aws-cli-2.x");
        assert_eq!(pr.base.r#ref, "master");
        assert_eq!(pr.base.sha.len(), 40);
        assert_eq!(pr.base.repo.name, "infra-charts");
        assert_eq!(pr.base.repo.id, 1219299223);
    }

    #[test]
    fn user_type_wire_values_are_pascal_case() {
        for (wire, matches) in [
            (
                "Bot",
                matches!(parse::<UserType>("Bot", &json!("Bot")), UserType::Bot),
            ),
            (
                "User",
                matches!(parse::<UserType>("User", &json!("User")), UserType::User),
            ),
            (
                "Organization",
                matches!(
                    parse::<UserType>("Organization", &json!("Organization")),
                    UserType::Organization
                ),
            ),
        ] {
            assert!(matches, "{wire} did not map to its own variant");
        }
    }

    #[test]
    fn user_type_rejects_snake_case() {
        for wire in ["bot", "user", "organization"] {
            assert!(
                serde_json::from_value::<UserType>(json!(wire)).is_err(),
                "{wire} must not deserialize"
            );
        }
    }

    #[test]
    fn actor_is_a_bot_and_repository_owner_is_an_organization() {
        let r = run();
        let actor: User = parse("actor", &r["actor"]);
        let owner: User = parse("repository.owner", &r["repository"]["owner"]);
        assert!(matches!(actor.user_type, UserType::Bot));
        assert!(matches!(owner.user_type, UserType::Organization));
        assert_eq!(actor.gravatar_id, "", "required but empty in practice");
        assert!(!actor.site_admin);
    }

    #[test]
    fn unknown_fields_are_ignored() {
        let r = run();
        assert!(
            r["actor"]
                .as_object()
                .unwrap()
                .contains_key("user_view_type"),
            "fixture should still contain the undocumented field this guards"
        );
        let mut actor = r["actor"].clone();
        actor
            .as_object_mut()
            .unwrap()
            .insert("some_future_field".into(), json!({"a": 1}));
        let _: User = parse("actor + unknown field", &actor);
    }

    #[test]
    fn repository_also_reads_a_full_repository_object() {
        let mut full = run()["repository"].clone();
        let obj = full.as_object_mut().unwrap();
        for (k, v) in [
            ("default_branch", json!("master")),
            ("visibility", json!("private")),
            ("archived", json!(false)),
            ("pushed_at", json!("2026-10-07T10:50:56Z")),
            ("topics", json!([])),
            ("stargazers_count", json!(0)),
        ] {
            obj.insert(k.into(), v);
        }
        let repo: Repository = parse("full repository", &full);
        assert_eq!(repo.full_name, "WMS-DEV/infra-charts");
        assert!(repo.private);
        assert!(!repo.fork);
        assert_eq!(
            repo.description.as_deref(),
            Some("WMS-DEV owned helmcharts")
        );
    }

    fn minimal_organization() -> Value {
        json!({
            "login": "WMS-DEV",
            "id": 118557818,
            "node_id": "O_kgDOB4-5Og",
            "description": null,
            "url": "https://api.github.com/orgs/WMS-DEV",
            "repos_url": "https://api.github.com/orgs/WMS-DEV/repos",
            "events_url": "https://api.github.com/orgs/WMS-DEV/events",
            "hooks_url": "https://api.github.com/orgs/WMS-DEV/hooks",
            "issues_url": "https://api.github.com/orgs/WMS-DEV/issues",
            "members_url": "https://api.github.com/orgs/WMS-DEV/members{/member}",
            "public_members_url": "https://api.github.com/orgs/WMS-DEV/public_members{/member}",
            "avatar_url": "https://example.invalid/avatar"
        })
    }

    #[test]
    fn organization_html_url_is_optional_but_url_is_not() {
        let org: Organization = parse("organization sans html_url", &minimal_organization());
        assert!(org.html_url.is_none());
        assert_eq!(org.login, "WMS-DEV");
        assert!(org.description.is_none(), "nullable, sent as null");

        let mut without_url = minimal_organization();
        without_url.as_object_mut().unwrap().remove("url");
        assert!(
            serde_json::from_value::<Organization>(without_url).is_err(),
            "`url` is required"
        );
    }

    #[test]
    fn installation_is_only_the_two_ids() {
        let inst: Installation = parse(
            "installation",
            &json!({"id": 93456789, "node_id": "MDIzOkluc3RhbGxhdGlvbg"}),
        );
        assert_eq!(inst.id, 93456789);
        assert_eq!(inst.node_id, "MDIzOkluc3RhbGxhdGlvbg");
    }
}
