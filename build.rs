use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use man::prelude::*;
use regex::Regex;
use yaml_rust::yaml::Hash;
use yaml_rust::{Yaml, YamlLoader};

fn s2y<T: ToString>(s: T) -> Yaml {
    Yaml::String(s.to_string())
}

fn parse_str<T: ToString>(a: &Hash, k: T) -> String {
    let s = a.get(&s2y(k));
    match s {
        Some(ele) => ele.as_str().unwrap().to_string(),
        None => "".to_string(),
    }
}

fn set_authors(p: Manual) -> Manual {
    let authors = env::var("CARGO_PKG_AUTHORS").unwrap();
    let re = Regex::new(r"([\w|\s]+)\s+<([a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z0-9]+)>").unwrap();
    re.captures_iter(&authors)
        .map(|x| {
            (
                x.get(1).map_or("", |m| m.as_str()),
                x.get(2).map_or("", |m| m.as_str()),
            )
        })
        .fold(p, |p, c| p.author(Author::new(c.0).email(c.1)))
}

fn set_args(mut p: Manual, y: &Yaml) -> Manual {
    let args = y["args"].as_vec().unwrap();
    for karg in args {
        let (key, val) = karg.as_hash().unwrap().iter().next().unwrap();
        let key = key.as_str().unwrap();
        let arg = val.as_hash().unwrap();
        if arg.get(&s2y("index")).is_none() {
            let takes_value = arg.get(&s2y("takes_value"));
            if takes_value.is_none() || !takes_value.unwrap().as_bool().unwrap() {
                let mut f = Flag::new()
                    .long(parse_str(&arg, "long").as_str())
                    .help(parse_str(&arg, "help").as_str());
                let short = parse_str(&arg, "short");
                if !short.is_empty() {
                    f = f.short(short.as_str());
                }
                p = p.flag(f);
            } else {
                let o_val = parse_str(&arg, "value_name");
                let key = if !o_val.is_empty() {
                    o_val.as_str()
                } else {
                    key
                };
                let mut o = Opt::new(key)
                    .long(parse_str(&arg, "long").as_str())
                    .help(parse_str(&arg, "help").as_str());
                let short = parse_str(&arg, "short");
                let default_value = parse_str(&arg, "default_value");
                if !short.is_empty() {
                    o = o.short(short.as_str());
                }
                if !default_value.is_empty() {
                    o = o.default_value(default_value.as_str());
                }
                p = p.option(o);
            }
        } else {
            p = p.arg(Arg::new(key));
        }
    }
    p
}

fn generate_manpage(yml: &Yaml, trg_dir: &Path) -> std::io::Result<()> {
    let out_location = trg_dir.join("git-req.1");

    let page = Manual::new("git-req").about(yml["about"].as_str().unwrap());
    let page = set_authors(page);
    let page = set_args(page, yml);

    let mut output = File::create(out_location)?;
    write!(output, "{}", page.render())?;

    Ok(())
}

fn main() -> std::io::Result<()> {
    let trg_dir = match env::var("CARGO_TARGET_DIR") {
        Ok(s) => s,
        Err(_) => String::from("target"),
    };
    let trg_dir = Path::new(&trg_dir).join(&env::var("PROFILE").unwrap());

    let yml = YamlLoader::load_from_str(include_str!("cli-flags.yml")).unwrap();
    let yml = yml.get(0).unwrap();

    generate_manpage(yml, &trg_dir)?;

    Ok(())
}
