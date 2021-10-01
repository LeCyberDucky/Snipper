// TODO: Consider duplicate snippet tags in source files
// TODO: Make output nicer for inactive snippets that are not overwritten

#![feature(bool_to_option)]
#![feature(format_args_capture)]
use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io::Write,
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
    comment: Option<String>,
    active: bool,
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
        begin: Option<&str>,
        content: Option<&str>,
        source_file: Option<PathBuf>,
        end: Option<&str>,
        comment: Option<&str>,
        active: bool,
        source: bool,
        latex: bool,
        extracted: bool,
    ) -> Result<Self> {
        let begin = begin.context("Snippet begin tag has no name.")?;
        let end = end.context("Snippet end tag has no name.")?;

        (begin == end)
            .then_some(Self {
                name: begin.to_owned(),
                content: content.map(|string| string.to_owned()),
                source_file,
                comment: comment.map(|string| string.to_owned()),
                active,
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
        .help("Root directory of LaTeX document"))
    .arg(Arg::with_name("Extract")
        .long("Extract")
        .takes_value(false)
        .case_insensitive(true)
        .help("Extract found snippets into separate snippet files for inclusion in LaTeX document"))
        .get_matches();

    let source_directory = Path::new(arguments.value_of("Source").unwrap());
    let target_directory = Path::new(arguments.value_of("Target").unwrap());
    let latex_directory = Path::new(arguments.value_of("LaTeX").unwrap());

    let source_files =
        files_with_extension(source_directory, vec!["cpp".into(), "h".into()], false);
    let tex_files = files_with_extension(latex_directory, vec!["tex".into()], false);
    let snippet_files = files_with_extension(target_directory, vec!["cpp".into()], false);

    // Finding snippets tagged as follows:
    /*
    // SNIPPET:BEGIN {some_cool_snippet_name}${}    (the ${} part is optional - for comments/descriptions)
    ...some_cool_snippet_content...
    // SNIPPET:END {some_cool_snippet_name}
    */
    let snippet_pattern_active = regex::RegexBuilder::new(
        r"(// SNIPPET:BEGIN \{(?P<BEGIN>.*?)\}(\$\{(?P<COMMENT>.*?)\})?(?P<SNIPPET>.*?)// SNIPPET:END \{(?P<END>.*?)\})",
    )
    .dot_matches_new_line(true)
    .build()
    .unwrap();

    let snippet_pattern_inactive = regex::RegexBuilder::new(
        r"(// _SNIPPET:BEGIN \{(?P<BEGIN>.*?)\}(\$\{(?P<COMMENT>.*?)\})?(?P<SNIPPET>.*?)// _SNIPPET:END \{(?P<END>.*?)\})",
    )
    .dot_matches_new_line(true)
    .build()
    .unwrap();

    // Finding snippet inclusions that look something like this:
    /*
    \lstinputlisting[...]{some_cool_snippet_name.cpp} where the [...] part is optional
    */
    let include_pattern =
        regex::RegexBuilder::new(r"(\\lstinputlisting.*?\{.*/(?P<SNIPPET_NAME>.*?)\.cpp.*?\})")
            .dot_matches_new_line(false)
            .build()
            .unwrap();

    let mut snippets = HashMap::new();
    for file in &source_files {
        let text = fs::read_to_string(&file).context(format!("{:?}", file));
        if let Ok(text) = text {
            for captures in snippet_pattern_active.captures_iter(&text) {
                let snippet = Snippet::new(
                    captures.name("BEGIN").map(|hit| hit.as_str()),
                    captures.name("SNIPPET").map(|hit| hit.as_str()),
                    Some(file.clone()),
                    captures.name("END").map(|hit| hit.as_str()),
                    captures.name("COMMENT").map(|hit| hit.as_str()),
                    true,
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

    for file in &source_files {
        let text = fs::read_to_string(&file).context(format!("{:#?}", file));
        if let Ok(text) = text {
            for captures in snippet_pattern_inactive.captures_iter(&text) {
                let snippet = Snippet::new(
                    captures.name("BEGIN").map(|hit| hit.as_str()),
                    captures.name("SNIPPET").map(|hit| hit.as_str()),
                    Some(file.clone()),
                    captures.name("END").map(|hit| hit.as_str()),
                    captures.name("COMMENT").map(|hit| hit.as_str()),
                    false,
                    true,
                    false,
                    false,
                )
                .context("Failed at creating snippet from LaTeX include statement.");

                if let Ok(snippet) = snippet {
                    snippets
                        .entry(snippet.name.clone())
                        .or_insert(snippet)
                        .active = false;
                } else {
                    eprint!("{:#?}", snippet);
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
                let snippet_name = captures.name("SNIPPET_NAME").map(|hit| hit.as_str());
                let snippet = Snippet::new(
                    snippet_name,
                    None,
                    None,
                    snippet_name,
                    None,
                    false,
                    false,
                    true,
                    false,
                )
                .context("Failed at creating snippet from LaTeX include statement.");

                if let Ok(snippet) = snippet {
                    snippets
                        .entry(snippet.name.clone())
                        .or_insert(snippet)
                        .latex = true;
                } else {
                    eprintln!("{:#?}", snippet);
                }
            }
        } else {
            eprintln!("{:#?}", text);
        }
    }

    for file in snippet_files {
        let snippet_name = file
            .file_stem()
            .map(|name| name.to_str().map(|name| name.to_owned()))
            .flatten()
            .context("Unable to obtain snippet name from snippet file.");
        if let Ok(snippet_name) = snippet_name {
            let snippet = Snippet::new(
                Some(&snippet_name),
                None,
                None,
                Some(&snippet_name),
                None,
                false,
                false,
                false,
                true,
            )
            .context("Failed at creating snippet from extracted snippet file.");

            if let Ok(snippet) = snippet {
                snippets
                    .entry(snippet.name.clone())
                    .or_insert(snippet)
                    .extracted = true;
            } else {
                eprintln!("{:#?}", snippet);
            }
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
    let state_width = "Act.: ❌, Src.: ❌, TeX.: ❌, Ext.: ❌".len();

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
        "{:count_width$}{spacer}{:snippet_name_width$}{spacer}|{:state_width$}  | Source file:",
        "", "Snippet name:", "State: "
    );

    let header_top = format!("{:1$}", " ", header.len());
    bunt::println!("{[underline]}", header_top);

    bunt::println!("{[underline]}", header);
    for (i, snippet) in snippets.iter().enumerate() {
        let file_name = snippet
            .source_file
            .as_ref()
            .and_then(|path| path.file_name().and_then(|name| name.to_str()))
            .unwrap_or("");
        let state = format!(
            "Act.: {}, Src.: {}, TeX.: {}, Ext.: {}",
            if snippet.active { "✔" } else { "❌" },
            if snippet.source { "✔" } else { "❌" },
            if snippet.latex { "✔" } else { "❌" },
            if snippet.extracted { "✔" } else { "❌" }
        );
        if snippet.source && snippet.latex && snippet.extracted {
            println!(
                "{:count_width$}{spacer}{:snippet_name_width$}{spacer}|{:state_width$}| {}",
                format!("{}.:", i + 1),
                snippet.name,
                state,
                file_name
            );
        } else {
            bunt::println!(
                "{2:0$}{1}{[red]4:3$}{1}|{6:5$}| {7}",
                count_width,
                spacer,
                format!("{}.:", i + 1),
                snippet_name_width,
                snippet.name,
                state_width,
                state,
                file_name
            );
        }
    }

    bunt::println!("{[underline]}", header_top);

    // 2. Update snippet files
    if !arguments.is_present("Extract") {
        return;
    }

    for snippet in snippets {
        if !snippet.source {
            bunt::println!(
                "{[red]}",
                format!(
                    "Snippet \"{}\" cannot be extracted. No associated source file.",
                    snippet.name
                )
            );
            continue;
        }

        let mut extraction_path = target_directory.join(&snippet.name);
        extraction_path.set_extension("cpp");

        let file = if snippet.active {
            OpenOptions::new()
                .write(true)
                .create(true)
                .open(&extraction_path)
                .map_err(|e| e.into())
        } else {
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&extraction_path)
                .context(format!("Snippet {} is inactive.", snippet.name))
        };

        if let Ok(mut file) = file {
            if let Some(content) = snippet.content {
                let write_status = file.write_all(content.as_bytes());
                if write_status.is_ok() {
                    bunt::println!(
                        "Snippet \"{}\" extracted to file \"{:?}\".",
                        snippet.name,
                        extraction_path.file_name()
                    );
                } else {
                    bunt::println!(
                        "Unable to write snippet file \"{:#?}\": {:#?}",
                        extraction_path.file_name(),
                        write_status
                    );
                }
            } else {
                bunt::println!(
                    "{[yellow]}",
                    format!(
                        "Snippet \"{}\" has no content. Empty file \"{:#?}\" created.",
                        snippet.name,
                        extraction_path.file_name()
                    )
                );
            }
        } else {
            eprintln!("Unable to write snippet to file: {:#?}", file);
            continue;
        }
    }
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
