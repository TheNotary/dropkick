use regex::Regex;
use serde_json::json;

use handlebars::{Handlebars, RenderError};

use crate::{config::Config, interpolation_config::ConfigBuilder};

pub fn render_template_with_handlebars(
    my_template: &str,
    repo_config: &Config,
) -> Result<String, RenderError> {
    // let re = Regex::new(r"#\{config\[:(\w+)\]\}").unwrap();
    let re = Regex::new(r"<%=\s*config\[\s*:(\w+)\s*\]\s*%>").unwrap();
    let my_template = re.replace_all(my_template, "{{$1}}").to_string();

    let reg = Handlebars::new();

    let context = ConfigBuilder::new(
        repo_config.project.name.clone(),
        repo_config.project.template.clone(),
    )
    .build()
    .expect("error: it went wrong");

    reg.render_template(&my_template, &json!(context))
}
