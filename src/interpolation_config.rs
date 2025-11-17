use std::process::Command;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct InterpolationConfig {
    pub name: String,
    pub title: String,
    pub unprefixed_name: String,
    pub unprefixed_pascal: String,
    pub underscored_name: String,
    pub pascal_name: String,
    pub camel_name: String,
    pub screamcase_name: String,
    pub namespaced_path: String,
    pub makefile_path: String,
    pub constant_name: String,
    pub constant_array: Vec<String>,
    pub author: String,
    pub email: String,
    pub git_repo_domain: String,
    pub git_repo_url: String,
    pub git_repo_path: String,
    pub image_path: String,
    pub registry_domain: String,
    pub registry_repo_path: String,
    pub k8s_domain: String,
    pub template: String,
    pub test: bool,
    pub ext: String,
    pub bin: bool,
}

pub struct ConfigBuilder {
    name: String,
    prefix: String,
    template: String,
    test: bool,
    ext: String,
    bin: bool,
}

impl ConfigBuilder {
    pub fn new(name: String, prefix: String) -> Self {
        Self {
            name,
            prefix,
            template: String::new(),
            test: false,
            ext: String::new(),
            bin: false,
        }
    }

    pub fn template(mut self, template: String) -> Self {
        self.template = template;
        self
    }

    pub fn test(mut self, test: bool) -> Self {
        self.test = test;
        self
    }

    pub fn ext(mut self, ext: String) -> Self {
        self.ext = ext;
        self
    }

    pub fn bin(mut self, bin: bool) -> Self {
        self.bin = bin;
        self
    }

    pub fn build(self) -> Result<InterpolationConfig, String> {
        let name = &self.name;

        // Title: "foo-bar-baz" -> "Foo Bar Baz"
        let title = name
            .replace('-', "_")
            .split('_')
            .map(|s| capitalize(s))
            .collect::<Vec<_>>()
            .join(" ");

        // Pascal name: "foo-bar" -> "FooBar"
        let pascal_name = name
            .replace('-', "_")
            .split('_')
            .map(|s| capitalize(s))
            .collect::<Vec<_>>()
            .join("");

        // Unprefixed name
        let unprefixed_name = if name.starts_with(&self.prefix) {
            name[self.prefix.len()..].to_string()
        } else {
            name.clone()
        };

        // Unprefixed pascal
        let unprefixed_pascal = unprefixed_name
            .replace('-', "_")
            .split('_')
            .map(|s| capitalize(s))
            .collect::<Vec<_>>()
            .join("");

        // Underscored name
        let underscored_name = name.replace('-', "_");

        // Constant name with :: separation
        let mut constant_name = name
            .split('_')
            .filter(|p| !p.is_empty())
            .map(|p| capitalize(p))
            .collect::<Vec<_>>()
            .join("");

        if constant_name.contains('-') {
            constant_name = constant_name
                .split('-')
                .map(|q| capitalize(q))
                .collect::<Vec<_>>()
                .join("::");
        }

        let constant_array: Vec<String> =
            constant_name.split("::").map(|s| s.to_string()).collect();

        // Git config values
        let git_user_name = get_git_config("user.name")?;
        let git_user_email = get_git_config("user.email").unwrap_or_default();
        let registry_domain = get_git_config("user.registry-domain").unwrap_or_default();
        let k8s_domain = get_git_config("user.k8s-domain").unwrap_or_default();

        let mut git_repo_domain = get_git_config("user.repo-domain").unwrap_or_default();
        if git_repo_domain.is_empty() {
            git_repo_domain = "github.com".to_string();
        }

        if git_user_name.is_empty() {
            return Err(
                "Error: git config user.name didn't return a value. You'll probably want to make sure that's configured with your github username:\n\ngit config --global user.name YOUR_GH_NAME".to_string()
            );
        }

        let git_repo_path =
            format!("{}/{}/{}", git_repo_domain, git_user_name, name).to_lowercase();
        let git_repo_url = format!("https://{}/{}/{}", git_repo_domain, git_user_name, name);
        let image_path = format!("{}/{}", git_user_name, name).to_lowercase();
        let registry_repo_path = format!("{}/{}", registry_domain, image_path).to_lowercase();

        let camel_name = if !pascal_name.is_empty() {
            let mut chars = pascal_name.chars();
            chars.next().unwrap().to_lowercase().collect::<String>() + chars.as_str()
        } else {
            String::new()
        };

        let screamcase_name = name.replace('-', "_").to_uppercase();
        let namespaced_path = name.replace('-', "/");
        let makefile_path = format!("{}/{}", underscored_name, underscored_name);

        let author = if git_user_name.is_empty() {
            "TODO: Write your name".to_string()
        } else {
            git_user_name
        };

        let email = if git_user_email.is_empty() {
            "TODO: Write your email address".to_string()
        } else {
            git_user_email
        };

        let k8s_domain = if k8s_domain.is_empty() {
            "k8s.domain.missing.from.gitconfig.local".to_string()
        } else {
            k8s_domain
        };

        Ok(InterpolationConfig {
            name: name.clone(),
            title,
            unprefixed_name,
            unprefixed_pascal,
            underscored_name,
            pascal_name,
            camel_name,
            screamcase_name,
            namespaced_path,
            makefile_path,
            constant_name,
            constant_array,
            author,
            email,
            git_repo_domain,
            git_repo_url,
            git_repo_path,
            image_path,
            registry_domain,
            registry_repo_path,
            k8s_domain,
            template: self.template,
            test: self.test,
            ext: self.ext,
            bin: self.bin,
        })
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

fn get_git_config(key: &str) -> Result<String, String> {
    let output = Command::new("git")
        .args(&["config", key])
        .output()
        .map_err(|e| format!("Failed to execute git command: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Ok(String::new())
    }
}
