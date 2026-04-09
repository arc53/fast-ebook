use std::path::{Path, PathBuf};
use std::process;

use clap::{Parser, Subcommand};

use _fast_ebook::model::EpubBook;
use _fast_ebook::reader::{read_epub_with_options, ReadOptions};
use _fast_ebook::validation::validate;

#[derive(Parser)]
#[command(
    name = "fast-ebook",
    version,
    about = "Fast EPUB inspection and validation tool"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Validate EPUB files against the spec
    Validate {
        /// EPUB files to validate
        #[arg(required = true)]
        files: Vec<PathBuf>,

        /// Output format: text or json
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// Print EPUB metadata
    Info {
        /// EPUB file to inspect
        file: PathBuf,

        /// Output format: text or json
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// Extract items from an EPUB
    Extract {
        /// EPUB file to extract from
        file: PathBuf,

        /// Output directory
        #[arg(long, short)]
        output_dir: PathBuf,

        /// Item type filter: all, images, documents, styles
        #[arg(long, default_value = "all")]
        r#type: String,
    },

    /// Convert an EPUB to Markdown
    Convert {
        /// EPUB file to convert
        file: PathBuf,

        /// Output file (default: stdout)
        #[arg(long, short)]
        output: Option<PathBuf>,
    },

    /// Scan a directory of EPUBs and print metadata
    Scan {
        /// Directory or files to scan
        #[arg(required = true)]
        paths: Vec<PathBuf>,

        /// Number of parallel workers
        #[arg(long, short, default_value = "4")]
        workers: usize,

        /// Output format: text, json, or csv
        #[arg(long, default_value = "text")]
        format: String,
    },
}

fn read_book(path: &Path) -> Result<EpubBook, String> {
    let path_str = path.to_string_lossy();
    read_epub_with_options(&path_str, &ReadOptions::default())
        .map_err(|e| format!("{}: {}", path_str, e))
}

fn cmd_validate(files: &[PathBuf], format: &str) {
    let mut all_valid = true;

    for file in files {
        let book = match read_book(file) {
            Ok(b) => b,
            Err(e) => {
                if format == "json" {
                    println!(
                        "{}",
                        serde_json::json!({"file": file.to_string_lossy(), "valid": false, "error": e})
                    );
                } else {
                    eprintln!("ERROR {}", e);
                }
                all_valid = false;
                continue;
            }
        };

        let issues = validate(&book);

        if format == "json" {
            println!(
                "{}",
                serde_json::json!({
                    "file": file.to_string_lossy(),
                    "valid": issues.is_empty(),
                    "issues": issues,
                })
            );
        } else if issues.is_empty() {
            println!("{}: VALID", file.display());
        } else {
            println!("{}: INVALID ({} issues)", file.display(), issues.len());
            for issue in &issues {
                println!("  - {}", issue);
            }
            all_valid = false;
        }
    }

    if !all_valid {
        process::exit(1);
    }
}

fn cmd_info(file: &Path, format: &str) {
    let book = match read_book(file) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("ERROR {}", e);
            process::exit(1);
        }
    };

    let get = |ns: &str, field: &str| -> Vec<String> {
        book.metadata
            .get(ns)
            .and_then(|m| m.get(field))
            .map(|items| items.iter().map(|i| i.value.clone()).collect())
            .unwrap_or_default()
    };

    if format == "json" {
        let info = serde_json::json!({
            "file": file.to_string_lossy(),
            "title": get("DC", "title"),
            "creator": get("DC", "creator"),
            "language": get("DC", "language"),
            "identifier": get("DC", "identifier"),
            "publisher": get("DC", "publisher"),
            "date": get("DC", "date"),
            "description": get("DC", "description"),
            "items": book.items.len(),
            "spine": book.spine.len(),
            "toc_entries": book.toc.len(),
        });
        println!("{}", serde_json::to_string_pretty(&info).unwrap());
    } else {
        println!("File:        {}", file.display());
        let titles = get("DC", "title");
        if !titles.is_empty() {
            println!("Title:       {}", titles.join(", "));
        }
        let creators = get("DC", "creator");
        if !creators.is_empty() {
            println!("Author:      {}", creators.join(", "));
        }
        let langs = get("DC", "language");
        if !langs.is_empty() {
            println!("Language:    {}", langs.join(", "));
        }
        let ids = get("DC", "identifier");
        if !ids.is_empty() {
            println!("Identifier:  {}", ids.join(", "));
        }
        let publishers = get("DC", "publisher");
        if !publishers.is_empty() {
            println!("Publisher:   {}", publishers.join(", "));
        }
        let dates = get("DC", "date");
        if !dates.is_empty() {
            println!("Date:        {}", dates.join(", "));
        }
        let descs = get("DC", "description");
        if !descs.is_empty() {
            println!("Description: {}", &descs[0][..descs[0].len().min(100)]);
        }
        println!("Items:       {}", book.items.len());
        println!("Spine:       {} entries", book.spine.len());
        println!("ToC:         {} entries", book.toc.len());
    }
}

fn cmd_extract(file: &Path, output_dir: &Path, type_filter: &str) {
    let book = match read_book(file) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("ERROR {}", e);
            process::exit(1);
        }
    };

    use _fast_ebook::item_type::ItemType;

    let type_match = |item_type: ItemType| -> bool {
        match type_filter {
            "all" => true,
            "images" => matches!(
                item_type,
                ItemType::Image | ItemType::Vector | ItemType::Cover
            ),
            "documents" => item_type == ItemType::Document,
            "styles" => item_type == ItemType::Style,
            _ => true,
        }
    };

    std::fs::create_dir_all(output_dir).unwrap_or_else(|e| {
        eprintln!("Failed to create output directory: {}", e);
        process::exit(1);
    });

    let canonical_output = output_dir
        .canonicalize()
        .unwrap_or_else(|_| output_dir.to_path_buf());

    let mut count = 0;
    for item in &book.items {
        if !type_match(item.item_type) {
            continue;
        }
        // Security: sanitize href to prevent path traversal (zip slip)
        let safe_href = item.href.replace('\\', "/");
        if safe_href.contains("..") || safe_href.starts_with('/') {
            eprintln!("Skipping item with unsafe path: {}", item.href);
            continue;
        }
        let out_path = canonical_output.join(&safe_href);
        // Double-check the resolved path is within output_dir
        if !out_path.starts_with(&canonical_output) {
            eprintln!("Skipping item escaping output directory: {}", item.href);
            continue;
        }
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::write(&out_path, item.get_content()).unwrap_or_else(|e| {
            eprintln!("Failed to write {}: {}", out_path.display(), e);
        });
        count += 1;
    }
    println!("Extracted {} items to {}", count, output_dir.display());
}

fn cmd_scan(paths: &[PathBuf], workers: usize, format: &str) {
    // Collect all .epub files
    let mut epub_files: Vec<PathBuf> = Vec::new();
    for path in paths {
        if path.is_dir() {
            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.extension().is_some_and(|e| e == "epub") {
                        epub_files.push(p);
                    }
                }
            }
        } else {
            epub_files.push(path.clone());
        }
    }

    let path_strings: Vec<String> = epub_files
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();
    let opts = ReadOptions {
        lazy: true,
        ..ReadOptions::default()
    };

    let results: Vec<_> = {
        use _fast_ebook::batch::read_epubs_parallel;
        read_epubs_parallel(&path_strings, &opts, Some(workers))
    };

    if format == "csv" {
        println!("file,title,author,language,items,spine,toc");
    }

    for (path, result) in epub_files.iter().zip(results) {
        match result {
            Ok(book) => {
                let title = book
                    .get_metadata_value("DC", "title")
                    .unwrap_or("")
                    .to_string();
                let author = book
                    .get_metadata_value("DC", "creator")
                    .unwrap_or("")
                    .to_string();
                let lang = book
                    .get_metadata_value("DC", "language")
                    .unwrap_or("")
                    .to_string();

                match format {
                    "json" => {
                        println!(
                            "{}",
                            serde_json::json!({
                                "file": path.to_string_lossy(),
                                "title": title,
                                "author": author,
                                "language": lang,
                                "items": book.items.len(),
                                "spine": book.spine.len(),
                                "toc": book.toc.len(),
                            })
                        );
                    }
                    "csv" => {
                        println!(
                            "\"{}\",\"{}\",\"{}\",{},{},{},{}",
                            path.display(),
                            title.replace('"', "\"\""),
                            author.replace('"', "\"\""),
                            lang,
                            book.items.len(),
                            book.spine.len(),
                            book.toc.len()
                        );
                    }
                    _ => {
                        println!(
                            "{}: \"{}\" by {} [{}, {} items]",
                            path.display(),
                            title,
                            author,
                            lang,
                            book.items.len()
                        );
                    }
                }
            }
            Err(e) => {
                eprintln!("ERROR {}: {}", path.display(), e);
            }
        }
    }
}

fn cmd_convert(file: &Path, output: Option<&PathBuf>) {
    let book = match read_book(file) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("ERROR {}", e);
            process::exit(1);
        }
    };

    let md = _fast_ebook::markdown::book_to_markdown(&book);

    if let Some(out_path) = output {
        std::fs::write(out_path, &md).unwrap_or_else(|e| {
            eprintln!("Failed to write {}: {}", out_path.display(), e);
            process::exit(1);
        });
        let title = book.get_metadata_value("DC", "title").unwrap_or("?");
        eprintln!(
            "Converted \"{}\" -> {} ({} bytes)",
            title,
            out_path.display(),
            md.len()
        );
    } else {
        print!("{}", md);
    }
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Validate { files, format } => cmd_validate(files, format),
        Commands::Info { file, format } => cmd_info(file, format),
        Commands::Extract {
            file,
            output_dir,
            r#type,
        } => cmd_extract(file, output_dir, r#type),
        Commands::Convert { file, output } => cmd_convert(file, output.as_ref()),
        Commands::Scan {
            paths,
            workers,
            format,
        } => cmd_scan(paths, *workers, format),
    }
}
