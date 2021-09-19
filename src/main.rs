#![feature(bool_to_option)]
#![feature(format_args_capture)]
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use clap::{App, Arg};
use walkdir::WalkDir;

#[derive(Debug, PartialEq, Eq)]
struct Snippet {
    name: String,
    content: Option<String>,
    source_file: Option<PathBuf>,
    source: bool,
    latex: bool,
    extracted: bool,
}

impl Ord for Snippet {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.name.cmp(&other.name)
    }
}

impl PartialOrd for Snippet {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.name.partial_cmp(&other.name)
    }
}

impl Snippet {
    fn new(
        begin: String,
        content: Option<String>,
        source_file: Option<PathBuf>,
        end: String,
        source: bool,
        latex: bool,
        extracted: bool,
    ) -> Result<Self> {
        (begin == end)
            .then_some(Self {
                name: begin.clone(),
                content,
                source_file,
                source,
                latex,
                extracted,
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

    let source_directory = Path::new(arguments.value_of("Source").unwrap());
    let target_directory = Path::new(arguments.value_of("Target").unwrap());
    let latex_directory = Path::new(arguments.value_of("LaTeX").unwrap());

    let source_files =
        files_with_extension(source_directory, vec!["cpp".into(), "h".into()], false);
    let tex_files = files_with_extension(latex_directory, vec!["tex".into()], false);
    let snippet_files = files_with_extension(target_directory, vec!["cpp".into()], false);

    // Finding snippets tagged as follows: 
    /* 
    // SNIPPET:BEGIN {some_cool_snippet_name}
    ...some_cool_snippet_content...
    // SNIPPET:END {some_cool_snippet_name}
    */
    let snippet_pattern = regex::RegexBuilder::new(
        r"(// SNIPPET:BEGIN \{(?P<BEGIN>.*?)\}(?P<SNIPPET>.*?)// SNIPPET:END \{(?P<END>.*?)\})",
    )
    .dot_matches_new_line(true)
    .build()
    .unwrap();

    // Finding snippet inclusions that look something like this: 
    /*
    \lstinputlisting{some_cool_snippet_name.cpp}
    */
    let include_pattern =
        regex::RegexBuilder::new(r"(\\lstinputlisting\{.*/(?P<SNIPPET_NAME>.*?)\.cpp.*?\})")
            .dot_matches_new_line(true)
            .build()
            .unwrap();

    let mut snippets = HashMap::new();
    for file in source_files {
        let text = fs::read_to_string(&file).context(format!("{:?}", file));
        if let Ok(text) = text {
            for captures in snippet_pattern.captures_iter(&text) {
                let snippet = Snippet::new(
                    captures["BEGIN"].into(),
                    Some(captures["SNIPPET"].into()),
                    Some(file.clone()),
                    captures["END"].into(),
                    true,
                    false,
                    false,
                );
                if let Ok(snippet) = snippet {
                    snippets.insert(snippet.name.clone(), snippet);
                } else {
                    eprintln!("{:#?}", snippet);
                }
            }
        } else {
            eprintln!("{:#?}", text);
        }
    }

    for file in tex_files {
        let text = fs::read_to_string(&file).context(format!("{:?}", file));
        if let Ok(text) = text {
            for captures in include_pattern.captures_iter(&text) {
                let snippet_name = &captures["SNIPPET_NAME"];
                snippets
                    .entry(snippet_name.to_owned())
                    .or_insert_with(|| {
                        Snippet::new(
                            snippet_name.to_owned(),
                            None,
                            None,
                            snippet_name.to_owned(),
                            false,
                            true,
                            false,
                        )
                        .expect("Failed at creating snippet from LaTeX include statement.")
                    })
                    .latex = true;
            }
        } else {
            eprintln!("{:#?}", text);
        }
    }

    for file in snippet_files {
        let snippet_name = file
            .file_name()
            .map(|name| name.to_str().map(|name| name.to_owned()))
            .flatten()
            .context("Unable to obtain snippet name from snippet file.");
        if let Ok(snippet_name) = snippet_name {
            snippets
                .entry(snippet_name.clone())
                .or_insert_with(|| {
                    Snippet::new(
                        snippet_name.to_owned(),
                        None,
                        None,
                        snippet_name.to_owned(),
                        false,
                        false,
                        true,
                    )
                    .expect("Failed at creating snippet from extracted snippet file.")
                })
                .extracted = true;
        } else {
            eprintln!("{:#?}", snippet_name);
        }
    }

    // 1. List found snippets and inclusions
    let mut snippets: Vec<_> = snippets.into_iter().map(|entry| entry.1).collect();
    snippets.sort();

    let mut count_width = 0;
    let mut snippet_name_width = "Snippet name:".len();
    let mut file_name_width = "Source file:".len();

    for (i, snippet) in snippets.iter().enumerate() {
        count_width = format!("{}.:", i + 1).len().max(count_width);
        snippet_name_width = snippet.name.len().max(snippet_name_width);
        let file_name = snippet
            .source_file
            .as_ref()
            .and_then(|path| path.file_name().and_then(|name| name.to_str()))
            .unwrap_or("");
        file_name_width = file_name.len().max(file_name_width);
    }

    let spacer = "    ";

    let header = format!(
        "{:count_width$}{spacer}{:snippet_name_width$}{spacer}  Source file:",
        "", "Snippet name:"
    );

    let header_top = format!("{:1$}", " ", header.len());
    bunt::println!("{[underline]}", header_top);
    
    bunt::println!("{[underline]}", header);
    // println!("{:_<width$}", "", width = header.len());
    for (i, snippet) in snippets.iter().enumerate() {
        let file_name = snippet
            .source_file
            .as_ref()
            .and_then(|path| path.file_name().and_then(|name| name.to_str()))
            .unwrap_or("");
        println!(
            "{:count_width$}{spacer}{:snippet_name_width$}{spacer}| {}",
            format!("{}.:", i + 1),
            snippet.name,
            file_name
        );
    }

    bunt::println!("{[underline]}", header_top);
}

fn files_with_extension(
    root_directory: &Path,
    mut extensions: Vec<String>,
    case_sensitive: bool,
) -> Vec<PathBuf> {
    if !case_sensitive {
        for x in extensions.iter_mut() {
            x.make_ascii_lowercase()
        }
    }

    WalkDir::new(root_directory)
        .into_iter()
        .filter_entry(|entry| {
            entry.file_type().is_dir()
                || (entry.file_type().is_file()
                    && extensions
                        .iter()
                        .map(|extension| Some(extension.to_string()))
                        .any(|x| {
                            x == entry
                                .path()
                                .extension()
                                .map(|extension| {
                                    if !case_sensitive {
                                        extension.to_owned().make_ascii_lowercase();
                                    }
                                    extension.to_str().map(|x| x.to_string())
                                })
                                .flatten()
                        }))
        })
        .collect::<Vec<_>>()
        .into_iter()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            entry.file_type().is_file().then_some(entry.into_path())
        })
        .collect()
}
