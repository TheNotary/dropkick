// use serde::Serialize;

use std::error::Error;
use tinytemplate::TinyTemplate;

use crate::interpolation_config::ConfigBuilder;

pub fn render_template(template: &str) -> Result<(), Box<dyn Error>> {
    let mut tt = TinyTemplate::new();
    tt.add_template("main", template)?;

    let context = ConfigBuilder::new("the name".to_string(), "the prefix".to_string())
        .build()
        .expect("error: it went wrong");

    let rendered = tt.render("main", &context)?;
    println!("{}", rendered);

    Ok(())
}
