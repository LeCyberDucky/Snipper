#![feature(bool_to_option)]
use std::{fs, path::Path};

use anyhow::{Context, Result};
use clap::{App, Arg};
use regex;
use walkdir::WalkDir;

#[derive(Debug)]
struct Snippet {
    name: String,
    content: String,
}

impl Snippet {
    fn new(begin: String, content: String, end: String) -> Result<Self> {
        (begin == end)
            .then_some(Self {
                name: begin.clone(),
                content,
            })
            .context(format!(
                "Snippet with mismatched begin and end tags\n\"{}\" != \"{}\"",
                begin, end
            ))
    }
}

fn main() {
    let arguments = App::new("Snipper").about("Collects snippets of code from source files into separate files for simple inclusion in LaTeX documents.")
    .arg(Arg::with_name("Source")
        .long("Source").value_name("DIRECTORY")
        .takes_value(true)
        .required(true)
        .validator(|path| Path::new(&path).is_dir().then_some(()).ok_or_else(||"Invalid source directory.".into()))
        .help("Root directory of source files"))
    .arg(Arg::with_name("Target")
        .long("Target")
        .value_name("DIRECTORY")
        .takes_value(true)
        .required(true)
        .validator(|path| Path::new(&path).is_dir().then_some(()).ok_or_else(||"Invalid target directory.".into()))
        .help("Directory, where snippets will be stored"))
    .arg(Arg::with_name("LaTeX")
        .long("LaTeX").value_name("DIRECTORY")
        .takes_value(true)
        .required(true)
        .validator(|path| Path::new(&path).is_dir().then_some(()).ok_or_else(||"Invalid LaTeX directory.".into()))
        .help("Root directory of LaTeX document")).get_matches();

    let snippet_pattern = regex::RegexBuilder::new(
        r"(// SNIPPET:BEGIN \{(?P<BEGIN>.*?)\}(?P<SNIPPET>.*?)// SNIPPET:END \{(?P<END>.*?)\})",
    )
    .dot_matches_new_line(true)
    .build()
    .unwrap();

    let source_directory = Path::new(arguments.value_of("Source").unwrap());
    let target_directory = Path::new(arguments.value_of("Target").unwrap());
    let latex_directory = Path::new(arguments.value_of("LaTeX").unwrap());

    let relevant_files: Vec<_> = WalkDir::new(source_directory)
        .into_iter()
        .filter_entry(|entry| {
            entry.file_type().is_dir()
                || (entry.file_type().is_file()
                    && [Some("cpp".into()), Some("h".into())].contains(
                        &entry
                            .path()
                            .extension()
                            .map(|extension| {
                                extension.to_str().map(|content| content.to_lowercase())
                            })
                            .flatten(),
                    ))
        })
        .collect::<Vec<_>>()
        .into_iter()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            entry.file_type().is_file().then_some(entry.into_path())
        })
        .collect();

    let snippets: Vec<Result<_, anyhow::Error>> = relevant_files
        .iter()
        .map(|file| {
            let text = fs::read_to_string(file)?;
            Ok(snippet_pattern
                .captures_iter(&text)
                .map(|captures| {
                    Snippet::new(
                        captures["BEGIN"].into(),
                        captures["SNIPPET"].into(),
                        captures["END"].into(),
                    )
                })
                .collect::<Vec<_>>())
        })
        .collect();

    // let snippets: Vec<_> = snippets.into_iter().map(|element| element.unwrap().into_iter()).collect();

    println!("{:#?}", snippets);
}
