use std::fmt;

use crate::errors::*;

use url::Url;

#[derive(Debug)]
pub enum GithubRepoInput {
    Url(String),
    Ssh(String),
}

/// Represents a GitHub repository that we can query things about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GithubRepo {
    /// The repository owner.
    pub owner: String,
    /// The repository name.
    pub name: String,
}

impl GithubRepo {
    /// Returns the domain. At the moment this is hardcoded to github.com, but
    /// it may support more options in the future.
    pub fn domain(&self) -> String {
        "https://github.com".to_owned()
    }

    /// Path component. Used with `domain` to construct `web_url`.
    pub fn web_path(&self) -> String {
        format!("/{}/{}", self.owner, self.name)
    }

    /// Returns a URL suitable for web access to the repository.
    pub fn web_url(&self) -> String {
        format!("{}{}", self.domain(), self.web_path())
    }

    /// Constructs a new Github repository from a "owner/name" string. Notably, this does not check
    /// whether the repo actually exists.
    pub fn from_url(repo_url: &str) -> Result<Self> {
        GithubRepoInput::new(repo_url.to_string())?.parse()
    }
}

impl fmt::Display for GithubRepo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}/{})", self.owner, self.name)
    }
}

impl GithubRepoInput {
    pub fn new(repo_string: String) -> Result<Self> {
        // Handle git+https just the same as https
        if repo_string.starts_with("https") || repo_string.starts_with("git+https") {
            Ok(Self::Url(repo_string))
        } else if repo_string.starts_with("git@") {
            Ok(Self::Ssh(repo_string))
        } else {
            let err = AxoprojectError::UnknownRepoStyle { url: repo_string };
            Err(err)
        }
    }

    pub fn parse(self) -> Result<GithubRepo> {
        match self {
            Self::Url(s) => Ok(Self::parse_url(s)?),
            Self::Ssh(s) => Ok(Self::parse_ssh(s)?),
        }
    }

    fn parse_url(repo_string: String) -> Result<GithubRepo> {
        let parsed = Url::parse(&repo_string)?;
        if parsed.domain() != Some("github.com") {
            return Err(AxoprojectError::NotGitHubError { url: repo_string });
        }
        let segment_list = parsed.path_segments().map(|c| c.collect::<Vec<_>>());
        if let Some(segments) = segment_list {
            if segments.len() >= 2 {
                let owner = segments[0].to_string();
                let name = Self::remove_git_suffix(segments[1].to_string());
                let rest_is_empty = segments.iter().skip(2).all(|s| s.trim().is_empty());
                if rest_is_empty {
                    return Ok(GithubRepo { owner, name });
                }
            }
        }
        Err(AxoprojectError::RepoParseError { repo: repo_string })
    }

    fn parse_ssh(repo_string: String) -> Result<GithubRepo> {
        let core = Self::remove_git_suffix(Self::remove_git_prefix(repo_string.clone())?);
        let segments: Vec<&str> = core.split('/').collect();
        if !segments.is_empty() && segments.len() >= 2 {
            let owner = segments[0].to_string();
            let name = Self::remove_git_suffix(segments[1].to_string());
            let rest_is_empty = segments.iter().skip(2).all(|s| s.trim().is_empty());
            if rest_is_empty {
                return Ok(GithubRepo { owner, name });
            }
        }
        Err(AxoprojectError::RepoParseError { repo: repo_string })
    }

    fn remove_git_prefix(s: String) -> Result<String> {
        let prefix = "git@github.com:";
        if let Some(stripped) = s.strip_prefix(prefix) {
            Ok(stripped.to_string())
        } else {
            Err(AxoprojectError::NotGitHubError { url: s })
        }
    }

    fn remove_git_suffix(s: String) -> String {
        if let Some(chomped) = s.strip_suffix(".git") {
            chomped.to_string()
        } else {
            s
        }
    }
}

/// Forgejo repository (generic for any Forgejo instance like Codeberg)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForgejoRepo {
    /// The Forgejo instance domain (e.g., "codeberg.org")
    pub domain: String,
    /// The repository owner
    pub owner: String,
    /// The repository name
    pub name: String,
}

impl ForgejoRepo {
    /// Returns the domain with https prefix
    pub fn domain(&self) -> String {
        format!("https://{}", self.domain)
    }

    /// Path component. Used with `domain` to construct `web_url`
    pub fn web_path(&self) -> String {
        format!("/{}/{}", self.owner, self.name)
    }

    /// Returns a URL suitable for web access to the repository
    pub fn web_url(&self) -> String {
        format!("{}{}", self.domain(), self.web_path())
    }

    /// Constructs a new Forgejo repository from a URL string
    pub fn from_url(repo_url: &str) -> Result<Self> {
        ForgejoRepoInput::new(repo_url.to_string())?.parse()
    }
}

impl fmt::Display for ForgejoRepo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}/{})", self.owner, self.name)
    }
}

/// Input for parsing Forgejo repository URLs
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForgejoRepoInput {
    /// An HTTPS URL like https://codeberg.org/owner/repo
    Url(String),
    /// An SSH URL like git@codeberg.org:owner/repo.git
    Ssh(String),
}

impl ForgejoRepoInput {
    pub fn new(repo_string: String) -> Result<Self> {
        if repo_string.starts_with("https") || repo_string.starts_with("git+https") {
            Ok(Self::Url(repo_string))
        } else if repo_string.starts_with("git@") {
            Ok(Self::Ssh(repo_string))
        } else {
            Err(AxoprojectError::NotForgejoError { url: repo_string })
        }
    }

    pub fn parse(&self) -> Result<ForgejoRepo> {
        match self {
            Self::Url(s) => Self::parse_https_url(s.clone()),
            Self::Ssh(s) => Self::parse_ssh_url(s.clone()),
        }
    }

    fn parse_https_url(s: String) -> Result<ForgejoRepo> {
        let s = Self::remove_git_suffix(s);
        let parsed = url::Url::parse(&s).map_err(|_| AxoprojectError::NotForgejoError { url: s.clone() })?;
        
        let domain = parsed.host_str().ok_or_else(|| AxoprojectError::NotForgejoError { url: s.clone() })?;
        
        // Parse the path which should be /owner/repo
        let path_segments: Vec<&str> = parsed.path().trim_matches('/').split('/').collect();
        if path_segments.len() != 2 {
            return Err(AxoprojectError::NotForgejoError { url: s });
        }

        Ok(ForgejoRepo {
            domain: domain.to_string(),
            owner: path_segments[0].to_string(),
            name: path_segments[1].to_string(),
        })
    }

    fn parse_ssh_url(s: String) -> Result<ForgejoRepo> {
        let s = Self::remove_git_suffix(s);
        
        // Parse SSH format: git@domain:owner/repo
        if let Some(at_pos) = s.find('@') {
            if let Some(colon_pos) = s[at_pos..].find(':') {
                let colon_pos = at_pos + colon_pos;
                let domain = &s[at_pos + 1..colon_pos];
                let path = &s[colon_pos + 1..];
                
                let path_parts: Vec<&str> = path.split('/').collect();
                if path_parts.len() == 2 {
                    return Ok(ForgejoRepo {
                        domain: domain.to_string(),
                        owner: path_parts[0].to_string(),
                        name: path_parts[1].to_string(),
                    });
                }
            }
        }
        
        Err(AxoprojectError::NotForgejoError { url: s })
    }

    fn remove_git_suffix(s: String) -> String {
        if let Some(chomped) = s.strip_suffix(".git") {
            chomped.to_string()
        } else {
            s
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_parses_an_https_repo_string() {
        let input = "https://github.com/axodotdev/oranda";
        let actual_owner = "axodotdev";
        let actual_name = "oranda";
        let parsed = GithubRepo::from_url(input).unwrap();
        assert_eq!(parsed.owner, actual_owner);
        assert_eq!(parsed.name, actual_name);
    }

    #[test]
    fn it_parses_an_https_repo_string_with_dot_git() {
        let input = "https://github.com/axodotdev/oranda.git";
        let actual_owner = "axodotdev";
        let actual_name = "oranda";
        let parsed = GithubRepo::from_url(input).unwrap();
        assert_eq!(parsed.owner, actual_owner);
        assert_eq!(parsed.name, actual_name);
    }

    #[test]
    fn it_parses_an_ssh_repo_string() {
        let input = "git@github.com:axodotdev/oranda.git";
        let actual_owner = "axodotdev";
        let actual_name = "oranda";
        let parsed = GithubRepo::from_url(input).unwrap();
        assert_eq!(parsed.owner, actual_owner);
        assert_eq!(parsed.name, actual_name);
    }
}
